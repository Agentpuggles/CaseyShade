use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;

const CSV_PATH: &str = "temperature_data.csv";

pub fn save_temperature(temperature: f32, humidity: f32, pressure: f32) {
    let needs_header = std::fs::metadata(CSV_PATH)
        .map(|m| m.len() == 0)
}
