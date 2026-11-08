pub mod sunlight;
pub mod temperature;
pub mod uv;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// True when the program should produce simulated readings instead of using
/// the I2C bus. Set `CASEYSHADE_FAKE_SENSORS=1` to enable it.
pub fn fake_sensors_enabled() -> bool {
    match std::env::var("CASEYSHADE_FAKE_SENSORS") {
        Ok(value) => value != "0",
        Err(_) => false,
    }
}

/// A number that changes with time and with every call, so simulated readings
/// move instead of repeating the same value.
fn next_step() -> f64 {
    static STEP: AtomicU64 = AtomicU64::new(0);

    let step = STEP.fetch_add(1, Ordering::Relaxed) as f64;

    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or(0.0);

    seconds * 0.05 + step * 0.35
}

pub fn fake_temperature() -> f32 {
    (22.0 + 3.0 * next_step().sin()) as f32
}

pub fn fake_humidity() -> f32 {
    (50.0 + 10.0 * (next_step() * 0.7).cos()) as f32
}

pub fn fake_pressure() -> f32 {
    (101_300.0 + 200.0 * next_step().sin()) as f32
}

pub fn fake_visible_light() -> u16 {
    (400.0 + 300.0 * next_step().sin().abs()) as u16
}
