use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;

const CSV_PATH: &str = "sensor_data.csv";

pub fn save_sensor_data(temperature: f32, humidity: f32, pressure: f32, visible_light : u16) -> std::io::Result<()>  {
    let needs_header = match std::fs::metadata(CSV_PATH){
        Ok(m) => m.len() == 0,
    Err(_) => true,
    };

    let mut save_sensor_data = OpenOptions::new()
        .create(true)
        .append(true)
        .open(CSV_PATH);

    if needs_header {
        writeln!(save_sensor_data, "Data, Temperature, Humidity, Pressure, Visible Light");
    }

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    writeln!(save_sensor_data,  "{},{},{},{},{}",
             timestamp,
             humidity,
             pressure,
             temperature,
             visible_light);

    Ok(())

}

