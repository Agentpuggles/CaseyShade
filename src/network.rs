use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::analysis::DailyStats;
use crate::data::{self, SensorSample};

/// Port the dashboard listens on when `CASEYSHADE_PORT` is not set.
const DEFAULT_PORT: u16 = 8080;

/// How long an accepted connection may take to send its request.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(5);

/// How many of the newest readings the dashboard page shows.
const SHOWN_ROWS: usize = 10;

/// A running dashboard. Stopping it shuts the listening socket down.
pub struct DashboardServer {
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl DashboardServer {
    /// Asks the server to stop listening and waits for its thread to finish.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);

        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// The port to listen on, from `CASEYSHADE_PORT` (8080 by default).
pub fn port() -> u16 {
    match std::env::var("CASEYSHADE_PORT") {
        Ok(value) => value.trim().parse().unwrap_or(DEFAULT_PORT),
        Err(_) => DEFAULT_PORT,
    }
}

/// Starts the dashboard on all interfaces so other computers on the network can
/// open it.
pub fn start(port: u16) -> Result<DashboardServer, String> {
    let listener = TcpListener::bind(("0.0.0.0", port))
        .map_err(|error| format!("could not listen on port {port}: {error}"))?;

    // The accept loop polls, which lets it notice a stop request.
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("could not set up the dashboard: {error}"))?;

    println!("Dashboard running at {}", address(port));
    println!("Open that address from a computer on the same network.");

    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);

    let worker = thread::Builder::new()
        .name("caseyshade-dashboard".to_string())
        .spawn(move || serve(listener, worker_stop))
        .map_err(|error| format!("could not start the dashboard: {error}"))?;

    Ok(DashboardServer {
        stop,
        worker: Some(worker),
    })
}

/// The address of this machine on the local network, for the printed hint.
fn address(port: u16) -> String {
    let local = std::net::UdpSocket::bind("0.0.0.0:0").and_then(|socket| {
        // Connecting a UDP socket sends nothing; it only picks the interface
        // that would be used, so its local address is the network address.
        socket.connect("8.8.8.8:80")?;
        socket.local_addr()
    });

    match local {
        Ok(address) => format!("http://{}:{port}/", address.ip()),
        Err(_) => format!("http://localhost:{port}/"),
    }
}

fn serve(listener: TcpListener, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                // One broken client must not take the dashboard down.
                let _ = handle(stream);
            }
            Err(_) => thread::sleep(Duration::from_millis(100)),
        }
    }
}

/// Answers one request. Only the request line matters: the path.
fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(CLIENT_TIMEOUT))?;

    let request_line = {
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        line
    };

    // "GET /path HTTP/1.1"
    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .split('?')
        .next()
        .unwrap_or("/");

    match path {
        "/" => respond(&mut stream, "200 OK", "text/html; charset=utf-8", page().as_bytes()),
        "/data.csv" => {
            let csv = std::fs::read_to_string(data::csv_path())
                .unwrap_or_else(|_| format!("{}\n", data::CSV_HEADER));

            respond_with(
                &mut stream,
                "200 OK",
                "text/csv; charset=utf-8",
                csv.as_bytes(),
                Some("attachment; filename=\"sensor_data.csv\""),
            )
        }
        "/api/readings" => respond(
            &mut stream,
            "200 OK",
            "application/json; charset=utf-8",
            readings_json().as_bytes(),
        ),
        _ => respond(
            &mut stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            b"Not found. Try /, /data.csv or /api/readings.\n",
        ),
    }
}

fn respond(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    respond_with(stream, status, content_type, body, None)
}

fn respond_with(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
    disposition: Option<&str>,
) -> std::io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n",
        body.len()
    );

    if let Some(disposition) = disposition {
        head.push_str(&format!("Content-Disposition: {disposition}\r\n"));
    }

    head.push_str("\r\n");

    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

/// The dashboard page: the newest readings and today's statistics.
fn page() -> String {
    let path = data::csv_path();
    let rows = data::read_samples(&path).unwrap_or_default();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let updated = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    html.push_str("<meta http-equiv=\"refresh\" content=\"5\">\n");
    html.push_str("<title>CaseyShade</title>\n<style>\n");
    html.push_str("body { font-family: system-ui, sans-serif; margin: 2rem; color: #222; }\n");
    html.push_str("table { border-collapse: collapse; margin-bottom: 1.5rem; }\n");
    html.push_str("th, td { padding: 0.35rem 0.9rem; border-bottom: 1px solid #ccc; text-align: right; }\n");
    html.push_str("th:first-child, td:first-child { text-align: left; }\n");
    html.push_str("th { background: #f3f3f3; }\n");
    html.push_str(".muted { color: #666; }\n");
    html.push_str("</style>\n</head>\n<body>\n<h1>CaseyShade</h1>\n");

    html.push_str(&format!(
        "<p class=\"muted\">Last update {updated}. Refreshed every 5 s. \
         <a href=\"/data.csv\">CSV download</a> &middot; <a href=\"/api/readings\">JSON</a></p>\n"
    ));

    html.push_str(&format!(
        "<h2>Latest readings</h2>\n<p class=\"muted\">{} saved in {}</p>\n",
        rows.len(),
        path.display()
    ));

    if rows.is_empty() {
        html.push_str("<p>No readings have been saved yet. Use option 5 on the Pi.</p>\n");
    } else {
        html.push_str(
            "<table>\n<tr><th>Time</th><th>Temperature (°C)</th><th>Humidity (%)</th>\
             <th>Pressure (Pa)</th><th>Light (raw)</th></tr>\n",
        );

        for row in rows.iter().rev().take(SHOWN_ROWS) {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                row.timestamp,
                decimal(row.sample.temperature),
                decimal(row.sample.humidity),
                decimal(row.sample.pressure),
                whole(row.sample.visible_light),
            ));
        }

        html.push_str("</table>\n");
    }

    html.push_str(&format!("<h2>Statistics for {today}</h2>\n"));

    let samples: Vec<SensorSample> = rows
        .iter()
        .filter(|row| row.date == today)
        .map(|row| row.sample.clone())
        .collect();

    match DailyStats::from_samples(&samples) {
        None => html.push_str("<p>No readings have been saved today yet.</p>\n"),
        Some(stats) => {
            html.push_str(&format!(
                "<p class=\"muted\">{} readings today</p>\n",
                stats.readings
            ));

            html.push_str(
                "<table>\n<tr><th>Reading</th><th>Count</th><th>Min</th><th>Max</th>\
                 <th>Average</th><th>Outliers</th></tr>\n",
            );

            let rows = [
                ("Temperature", stats.temperature.as_ref()),
                ("Humidity (%)", stats.humidity.as_ref()),
                ("Pressure (Pa)", stats.pressure.as_ref()),
                ("Light (raw)", stats.sunlight.as_ref()),
            ];

            for (name, field) in rows {
                match field {
                    Some(field) => html.push_str(&format!(
                        "<tr><td>{name}</td><td>{}</td><td>{:.2}</td><td>{:.2}</td><td>{:.2}</td><td>{}</td></tr>\n",
                        field.count,
                        field.min,
                        field.max,
                        field.average_without_outliers,
                        field.outliers_removed
                    )),
                    None => html.push_str(&format!("<tr><td>{name}</td><td>-</td></tr>\n")),
                }
            }

            html.push_str("</table>\n");
        }
    }

    html.push_str("</body>\n</html>\n");

    html
}

/// Every saved reading as JSON, newest last.
fn readings_json() -> String {
    let rows = data::read_samples(&data::csv_path()).unwrap_or_default();

    let mut json = String::from("[");

    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }

        json.push_str(&format!(
            "{{\"timestamp\":\"{}\",\"temperature\":{},\"humidity\":{},\"pressure\":{},\"visible_light\":{}}}",
            row.timestamp,
            json_number(row.sample.temperature.map(f64::from)),
            json_number(row.sample.humidity.map(f64::from)),
            json_number(row.sample.pressure.map(f64::from)),
            json_number(row.sample.visible_light.map(f64::from)),
        ));
    }

    json.push(']');

    json
}

fn json_number(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("{value:.2}"),
        None => "null".to_string(),
    }
}

fn decimal(value: Option<f32>) -> String {
    match value {
        Some(value) => format!("{value:.2}"),
        None => "-".to_string(),
    }
}

fn whole(value: Option<u16>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "-".to_string(),
    }
}
