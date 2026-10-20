
use linux_embedded_hal as hal;
use hal::{Delay, I2cdev};
use embedded_hal::delay::DelayNs;
use si1145::Si1145;

pub fn show_sunlight() {
    println!("Connecting to sunlight sensor...");

    let dev = match I2cdev::new("/dev/i2c-1") {
        Ok(dev) => dev,
        Err(error) => {
            println!("Could not connect: {error}");
            return;
        }
    };

    let mut delay = Delay;

    let mut si1145 = match Si1145::new(dev, &mut delay) {
        Ok(sensor) => sensor,
        Err(error) => {
            println!("Could not start sensor: {error:?}");
            return;
        }
    };

    println!("Reading sunlight...");

    loop {
        match si1145.measurement_ready() {
            Ok(true) => {
                let lux = match si1145.measure_lux() {
                    Ok(value) => value,
                    Err(error) => {
                        println!("Could not read light: {error:?}");
                        return;
                    }
                };

                let uv_index = match si1145.measure_uv_index() {
                    Ok(value) => value,
                    Err(error) => {
                        println!("Could not read UV: {error:?}");
                        return;
                    }
                };

                println!("Ambient light: {lux} lux");
                println!("UV Index: {uv_index:.2}");
            }

            Ok(false) => {
                println!("Waiting for a reading...");
            }

            Err(error) => {
                println!("Could not get a reading: {error:?}");
                return;
            }
        }

        delay.delay_ms(1000_u32);
    }
}
