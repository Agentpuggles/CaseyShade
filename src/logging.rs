use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use crate::data::{self, SensorSample};
use crate::graph::SensorReading;
use crate::sensors;

/// Seconds between saved readings when `CASEYSHADE_INTERVAL_SECS` is not set.
const DEFAULT_INTERVAL_SECS: u64 = 60;

/// How often the background thread looks up from waiting to check whether it
/// should stop, so the menu does not hang on "stop saving readings".
const WAKE_STEP: Duration = Duration::from_millis(100);

/// A running background collection of readings. Dropping it without calling
/// `stop` leaves the thread running until the program exits.
pub struct StreamHandle {
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl StreamHandle {
    /// Asks the background thread to finish and waits for it to stop.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);

        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// The time between readings, from `CASEYSHADE_INTERVAL_SECS` (60 s by default).
pub fn collection_interval() -> Duration {
    match std::env::var("CASEYSHADE_INTERVAL_SECS") {
        Ok(value) => match value.trim().parse::<u64>() {
            Ok(seconds) if seconds > 0 => Duration::from_secs(seconds),
            _ => Duration::from_secs(DEFAULT_INTERVAL_SECS),
        },
        Err(_) => Duration::from_secs(DEFAULT_INTERVAL_SECS),
    }
}

/// Starts collecting readings in the background.
///
/// Every `interval` one set of readings is taken. When `save` is set the
/// readings are appended to the CSV file, and when `sender` is given they are
/// also handed to the graph window.
pub fn start_stream(
    interval: Duration,
    save: bool,
    sender: Option<Sender<SensorReading>>,
) -> Result<StreamHandle, String> {
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);

    let worker = thread::Builder::new()
        .name("caseyshade-collector".to_string())
        .spawn(move || run(interval, save, sender, worker_stop))
        .map_err(|error| format!("could not start the background collector: {error}"))?;

    Ok(StreamHandle {
        stop,
        worker: Some(worker),
    })
}

/// The collection loop: take a reading, save it, then wait for the next one.
fn run(
    interval: Duration,
    save: bool,
    sender: Option<Sender<SensorReading>>,
    stop: Arc<AtomicBool>,
) {
    let mut sensors = OpenSensors::connect();

    while !stop.load(Ordering::Relaxed) {
        let sample = sensors.sample();

        if let Some(sender) = &sender {
            // A closed channel means the graph window has been closed.
            if sender.send(reading(&sample)).is_err() {
                break;
            }
        }

        if save {
            if let Err(error) = data::save_sensor_data_to(&data::csv_path(), &sample) {
                println!("Could not save a reading: {error}");
            }
        }

        wait(interval, &stop);
    }
}

/// Sleeps for `interval`, returning early once a stop has been requested.
fn wait(interval: Duration, stop: &AtomicBool) {
    let deadline = Instant::now() + interval;

    loop {
        let now = Instant::now();

        if now >= deadline || stop.load(Ordering::Relaxed) {
            return;
        }

        thread::sleep(WAKE_STEP.min(deadline - now));
    }
}

/// Converts a sample into the reading the graphs plot.
fn reading(sample: &SensorSample) -> SensorReading {
    use chrono::Timelike;

    let now = chrono::Local::now();

    SensorReading {
        time: f64::from(now.hour())
            + f64::from(now.minute()) / 60.0
            + f64::from(now.second()) / 3600.0,
        temperature: sample.temperature.map(f64::from).unwrap_or(f64::NAN),
        humidity: sample.humidity.map(f64::from).unwrap_or(f64::NAN),
        pressure: sample.pressure.map(f64::from).unwrap_or(f64::NAN),
        sunlight: sample.visible_light.map(f64::from).unwrap_or(f64::NAN),
    }
}

/// The sensors the background thread keeps open between readings, so a chip is
/// not reset for every sample.
struct OpenSensors {
    temperature: Option<sensors::temperature::TemperatureSensor>,
    sunlight: Option<sensors::sunlight::SunlightSensor>,
}

impl OpenSensors {
    fn connect() -> Self {
        if sensors::fake_sensors_enabled() {
            return Self {
                temperature: None,
                sunlight: None,
            };
        }

        let temperature = match sensors::temperature::TemperatureSensor::new() {
            Ok(sensor) => Some(sensor),
            Err(error) => {
                println!("Temperature sensor unavailable: {error}");
                None
            }
        };

        let sunlight = match sensors::sunlight::SunlightSensor::new() {
            Ok(sensor) => Some(sensor),
            Err(error) => {
                println!("Sunlight sensor unavailable: {error}");
                None
            }
        };

        Self {
            temperature,
            sunlight,
        }
    }

    /// One set of readings. A value stays `None` when that sensor failed.
    fn sample(&mut self) -> SensorSample {
        if sensors::fake_sensors_enabled() {
            return SensorSample {
                temperature: Some(sensors::fake_temperature()),
                humidity: Some(sensors::fake_humidity()),
                pressure: Some(sensors::fake_pressure()),
                visible_light: Some(sensors::fake_visible_light()),
            };
        }

        let mut sample = SensorSample::default();

        if let Some(sensor) = self.temperature.as_mut() {
            if let Ok(reading) = sensor.read() {
                sample.temperature = Some(reading.temperature);
                sample.humidity = Some(reading.humidity);
                sample.pressure = Some(reading.pressure);
            }
        }

        if let Some(sensor) = self.sunlight.as_mut() {
            sample.visible_light = sensor.read_visible().ok();
        }

        sample
    }
}

/// Checks each sensor and prints what it reported.
pub fn show_sensor_status() {
    if sensors::fake_sensors_enabled() {
        println!("CaseyShade is using simulated readings (CASEYSHADE_FAKE_SENSORS).");
        println!("The I2C sensors are not being read.");
        return;
    }

    println!("Checking the sensors on /dev/i2c-1");
    println!();

    match sensors::temperature::TemperatureSensor::new() {
        Ok(mut sensor) => match sensor.read() {
            Ok(reading) => println!(
                "Temperature, humidity and pressure: {:.2} °C, {:.2} %, {:.2} Pa",
                reading.temperature, reading.humidity, reading.pressure
            ),
            Err(error) => println!("Temperature, humidity and pressure: {error}"),
        },
        Err(error) => println!("Temperature, humidity and pressure: {error}"),
    }

    match sensors::sunlight::SunlightSensor::new() {
        Ok(mut sensor) => match sensor.read_visible() {
            Ok(value) => println!("Visible light: raw reading {value}"),
            Err(error) => println!("Visible light: {error}"),
        },
        Err(error) => println!("Visible light: {error}"),
    }

    println!();
    println!("If a sensor is missing, check its wiring with: i2cdetect -y 1");
}
