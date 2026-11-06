use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;

const CSV_PATH: &str = "temperature_data.csv";

pub fn save_temperature(temperature: f32, humidity: f32, pressure: f32) -> std::io::Result<()>  {
    let needs_header = match std::fs::metadata(CSV_PATH){
        Ok(m) => m.len() == 0,
    Err(_) => true,};
    let mut temp = OpenOptions::new()
        .create(true)
        .append(true)
        .open(CSV_PATH);

    if needs_header {
        writeln!(temp, "Data, Temperature, Humidity, Pressure");
    }

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(temp,  "{},{},{},{}",
             timestamp,
             humidity,
             pressure,
             temperature);

    Ok(())

}

