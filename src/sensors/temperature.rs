use bme280::i2c::BME280;
use linux_embedded_hal::{Delay, I2cdev};

/// One reading from the temperature, humidity and pressure sensor.
pub struct TemperatureReading {
    pub temperature: f32,
    pub humidity: f32,
    pub pressure: f32,
}

/// A connected BME280 on the Raspberry Pi's I2C bus.
pub struct TemperatureSensor {
    sensor: BME280<I2cdev>,
    delay: Delay,
}

impl TemperatureSensor {
    /// Opens `/dev/i2c-1`, connects to the sensor at address 0x76 and
    /// initialises it.
    pub fn new() -> Result<Self, String> {
        let i2c = I2cdev::new("/dev/i2c-1")
            .map_err(|error| format!("could not open /dev/i2c-1: {error:?}"))?;

        let mut sensor = BME280::new_primary(i2c);
        let mut delay = Delay;

        sensor
            .init(&mut delay)
            .map_err(|error| format!("could not initialise the BME280: {error:?}"))?;

        Ok(Self { sensor, delay })
    }

    /// Takes one measurement.
    pub fn read(&mut self) -> Result<TemperatureReading, String> {
        self.sensor
            .measure(&mut self.delay)
            .map(|reading| TemperatureReading {
                temperature: reading.temperature,
                humidity: reading.humidity,
                pressure: reading.pressure,
            })
            .map_err(|error| format!("could not read the BME280: {error:?}"))
    }
}

/// Connects to the sensor and prints one reading.
pub fn show_temperature() {
    println!("Connecting to temperature sensor...");

    let mut sensor = match TemperatureSensor::new() {
        Ok(sensor) => sensor,
        Err(error) => {
            println!("Could not use the sensor: {error}");
            return;
        }
    };

    match sensor.read() {
        Ok(readings) => {
            println!();
            println!("--- Temperature Readings ---");
            println!("Temperature: {:.2} °C", readings.temperature);
            println!("Humidity:    {:.2} %", readings.humidity);
            println!("Pressure:    {:.2} Pa", readings.pressure);
            println!("----------------------------");
        }
        Err(error) => println!("Could not read the sensor: {error}"),
    }
}
