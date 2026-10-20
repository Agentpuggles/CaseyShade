
use bme280::i2c::BME280;
use linux_embedded_hal::{Delay, I2cdev};

pub fn show_temperature() {
    println!("Connecting to temperature sensor...");

    // Connect to I2C bus 1 on the Raspberry Pi.
    let i2c = match I2cdev::new("/dev/i2c-1") {
        Ok(i2c) => i2c,
        Err(error) => {
            println!("Could not connect to the sensor: {error}");
            return;
        }
    };

    // Create the sensor using I2C address 0x76.
    let mut sensor = BME280::new_primary(i2c);

    // The sensor needs a delay while it starts and takes readings.
    let mut delay = Delay;

    // Initialise the sensor.
    if let Err(error) = sensor.init(&mut delay) {
        println!("Could not initialise the sensor: {error:?}");
        return;
    }

    // Read temperature, humidity and pressure.
    match sensor.measure(&mut delay) {
        Ok(readings) => {
            println!();
            println!("--- Temperature Readings ---");
            println!("Temperature: {:.2} °C", readings.temperature);
            println!("Humidity:    {:.2} %", readings.humidity);
            println!("Pressure:    {:.2} Pa", readings.pressure);
            println!("----------------------------");
        }
        Err(error) => {
            println!("Could not read the sensor: {error:?}");
        }
    }
}
