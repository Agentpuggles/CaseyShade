use chrono::{Local, NaiveDateTime, Timelike};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Default file the readings are saved to.
pub const CSV_PATH: &str = "sensor_data.csv";

/// Column order of the saved rows, in the order they are written.
pub const CSV_HEADER: &str = "Data, Humidity, Pressure, Temperature, Visible Light";

/// One set of readings. A value is `None` when that sensor had no reading.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SensorSample {
    pub temperature: Option<f32>,
    pub humidity: Option<f32>,
    pub pressure: Option<f32>,
    pub visible_light: Option<u16>,
}

/// One row read back from the CSV file.
pub struct StoredSample {
    pub timestamp: String,
    /// The date part of the timestamp, for example "2026-11-08".
    pub date: String,
    /// Hours since midnight, used as the x axis of the graphs.
    pub hour: f64,
    pub sample: SensorSample,
}

/// The path readings are saved to. `CASEYSHADE_CSV` overrides it for tests.
pub fn csv_path() -> PathBuf {
    match std::env::var("CASEYSHADE_CSV") {
        Ok(path) if !path.is_empty() => PathBuf::from(path),
        _ => PathBuf::from(CSV_PATH),
    }
}

/// Appends one timestamped row, writing the header first when the file is new.
pub fn save_sensor_data_to(path: &Path, sample: &SensorSample) -> std::io::Result<()> {
    let needs_header = match std::fs::metadata(path) {
        Ok(metadata) => metadata.len() == 0,
        Err(_) => true,
    };

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;

    if needs_header {
        writeln!(file, "{CSV_HEADER}")?;
    }

    writeln!(
        file,
        "{},{},{},{},{}",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        format_number(sample.humidity),
        format_number(sample.pressure),
        format_number(sample.temperature),
        match sample.visible_light {
            Some(value) => value.to_string(),
            None => "-".to_string(),
        }
    )?;

    Ok(())
}

/// Reads every saved row. A missing file is treated as "no readings yet".
pub fn read_samples(path: &Path) -> std::io::Result<Vec<StoredSample>> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };

    let mut rows = Vec::new();

    // The first line is the header written by `save_sensor_data_to`.
    for line in contents.lines().skip(1) {
        let fields: Vec<&str> = line.split(',').collect();

        if fields.len() < 5 {
            continue;
        }

        let timestamp = fields[0].trim().to_string();

        let (date, hour) = match NaiveDateTime::parse_from_str(&timestamp, "%Y-%m-%d %H:%M:%S") {
            Ok(date_time) => (
                date_time.format("%Y-%m-%d").to_string(),
                date_time.hour() as f64
                    + date_time.minute() as f64 / 60.0
                    + date_time.second() as f64 / 3600.0,
            ),
            Err(_) => continue,
        };

        rows.push(StoredSample {
            timestamp,
            date,
            hour,
            sample: SensorSample {
                humidity: parse_f32(fields[1]),
                pressure: parse_f32(fields[2]),
                temperature: parse_f32(fields[3]),
                visible_light: parse_u16(fields[4]),
            },
        });
    }

    Ok(rows)
}

fn format_number(value: Option<f32>) -> String {
    match value {
        Some(value) => format!("{value:.2}"),
        None => "-".to_string(),
    }
}

fn parse_f32(field: &str) -> Option<f32> {
    let field = field.trim();

    if field.is_empty() || field == "-" {
        None
    } else {
        field.parse().ok()
    }
}

fn parse_u16(field: &str) -> Option<u16> {
    let field = field.trim();

    if field.is_empty() || field == "-" {
        None
    } else {
        field.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_csv(name: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let id = COUNTER.fetch_add(1, Ordering::Relaxed);

        std::env::temp_dir().join(format!("caseyshade-data-{}-{name}-{id}.csv", std::process::id()))
    }

    #[test]
    fn writes_a_header_then_a_readable_row() {
        let path = temp_csv("roundtrip");
        let _ = std::fs::remove_file(&path);

        let sample = SensorSample {
            temperature: Some(21.5),
            humidity: Some(48.25),
            pressure: Some(101_325.0),
            visible_light: Some(512),
        };

        save_sensor_data_to(&path, &sample).unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents.lines().next().unwrap(), CSV_HEADER);

        let rows = read_samples(&path).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].sample, sample);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_missing_value_is_written_as_a_dash_and_read_back_as_none() {
        let path = temp_csv("missing");
        let _ = std::fs::remove_file(&path);

        let sample = SensorSample {
            temperature: None,
            humidity: Some(50.0),
            pressure: None,
            visible_light: None,
        };

        save_sensor_data_to(&path, &sample).unwrap();

        let rows = read_samples(&path).unwrap();
        assert_eq!(rows[0].sample.temperature, None);
        assert_eq!(rows[0].sample.visible_light, None);

        let _ = std::fs::remove_file(&path);
    }
}
