use embedded_hal::i2c::I2c;
use linux_embedded_hal::I2cdev;
use std::thread;
use std::time::Duration;

// ---- Hardware constants (from the Si115x datasheet) ----
const ADDRESS: u8 = 0x53; // Si1151 default I2C address

// Registers
const PART_ID: u8 = 0x00; // Device identification register
const HOSTIN_0: u8 = 0x0A; // Input register
const COMMAND: u8 = 0x0B; // Command register (RESET_SW, FORCE, PARAM_SET)
const RESPONSE_0: u8 = 0x11; // Status register
const HOSTOUT_2: u8 = 0x15; // Visible channel, high byte
const HOSTOUT_3: u8 = 0x16; // Visible channel, low byte

// Command codes
const RESET_SW: u8 = 0x01; // Reboot the chip
const FORCE: u8 = 0x11; // Take a measurement right now
const PARAM_SET: u8 = 0x80; // Adding parameters to an address

// Global parameter table: which channels a FORCE command measures
const CHAN_LIST: u8 = 0x01;

// Channel 0: infrared photodiode
const ADCCONFIG_0: u8 = 0x02;
const ADCSENS_0: u8 = 0x03; // Sensitivity
const ADCPOST_0: u8 = 0x04; // Output post processing
const MEASCONFIG_0: u8 = 0x05; // When and how often to measure

// Channel 1: visible-light photodiode
const ADCCONFIG_1: u8 = 0x06;
const ADCSENS_1: u8 = 0x07;
const ADCPOST_1: u8 = 0x08;
const MEASCONFIG_1: u8 = 0x09;

/// How many times the driver waits for the sensor before giving up. Each wait
/// is 200 microseconds, so a command gets about 50 milliseconds in total.
const ACK_ATTEMPTS: u32 = 250;

/// What RESPONSE_0 reads once the chip has finished starting up: sleeping, no
/// command error, and a command counter of 0x0F.
const RESPONSE_READY: u8 = 0x2F;

/// The Si115x does not answer on I2C while it restarts after RESET_SW. The
/// datasheet asks the host to wait for the start-up time before talking to the
/// chip again, which is well under a millisecond; this leaves a wide margin.
const STARTUP_DELAY: Duration = Duration::from_millis(25);

/// How many 1 ms attempts the reset check makes before giving up. A read that
/// is not acknowledged means "still starting up", not "sensor missing".
const RESET_ATTEMPTS: u32 = 500;

/// I2C reads in two steps: write the register number ("I want register x"),
/// then read the reply. `write_read` does both in one transaction.
fn read_byte(i2c: &mut I2cdev, reg: u8) -> Result<u8, String> {
    let mut buf = [0u8; 1];

    i2c.write_read(ADDRESS, &[reg], &mut buf)
        .map_err(|error| format!("I2C read of register 0x{reg:02X} failed: {error:?}"))?;

    Ok(buf[0])
}

/// Writes one byte to a sensor register over I2C.
fn write_byte(i2c: &mut I2cdev, reg: u8, value: u8) -> Result<(), String> {
    i2c.write(ADDRESS, &[reg, value])
        .map_err(|error| format!("I2C write to register 0x{reg:02X} failed: {error:?}"))
}

/// Waits until the sensor's response counter changes to the expected value.
/// The counter is four bits, so it wraps from 15 back to 0.
fn wait_for_ack(i2c: &mut I2cdev, previous_response: u8) -> Result<(), String> {
    let expected = previous_response.wrapping_add(1) & 0x0F;

    for _ in 0..ACK_ATTEMPTS {
        if (read_byte(i2c, RESPONSE_0)? & 0x0F) == expected {
            return Ok(());
        }

        thread::sleep(Duration::from_micros(200));
    }

    Err("the Si1151 did not acknowledge a command".to_string())
}

/// Tells the sensor to store `value` in the parameter identified by `location`.
fn param_set(i2c: &mut I2cdev, location: u8, value: u8) -> Result<(), String> {
    let before = read_byte(i2c, RESPONSE_0)?;

    write_byte(i2c, HOSTIN_0, value)?;
    write_byte(i2c, COMMAND, location | PARAM_SET)?;

    wait_for_ack(i2c, before)
}

/// Sends a command such as RESET_SW or FORCE.
fn send_command(i2c: &mut I2cdev, code: u8) -> Result<(), String> {
    let before = read_byte(i2c, RESPONSE_0)?;

    write_byte(i2c, COMMAND, code)?;

    wait_for_ack(i2c, before)
}

/// Asks for one measurement and returns the raw 16-bit visible-light value.
/// The result is a raw sensor count, not a measurement in lux.
fn read_visible_raw(i2c: &mut I2cdev) -> Result<u16, String> {
    send_command(i2c, FORCE)?;

    let hi = read_byte(i2c, HOSTOUT_2)?;
    let lo = read_byte(i2c, HOSTOUT_3)?;

    Ok(((hi as u16) << 8) | (lo as u16))
}

// ---------- Public API ----------

/// A connected and configured Si1151.
pub struct SunlightSensor {
    i2c: I2cdev,
}

impl SunlightSensor {
    /// Opens the I2C bus, checks the part ID, resets the chip and configures
    /// channel 0 for infrared and channel 1 for visible light.
    pub fn new() -> Result<Self, String> {
        let mut i2c = I2cdev::new("/dev/i2c-1")
            .map_err(|error| format!("could not open /dev/i2c-1: {error:?}"))?;

        // The Si1151 reports part ID 0x51. A different value means we are not
        // talking to the expected chip.
        let id = read_byte(&mut i2c, PART_ID)?;

        if id != 0x51 {
            return Err(format!("unexpected part ID 0x{id:02X} (expected 0x51)"));
        }

        write_byte(&mut i2c, COMMAND, RESET_SW)?;

        // The chip stops acknowledging its I2C address while it restarts, which
        // the kernel reports as EREMOTEIO (errno 121). Wait out the start-up
        // delay first, then treat an unacknowledged read as "not ready yet" so
        // only the timeout counts as a failure.
        thread::sleep(STARTUP_DELAY);

        let mut attempts = 0;
        let mut ready = false;

        while !ready {
            if attempts >= RESET_ATTEMPTS {
                return Err("the Si1151 did not finish resetting".to_string());
            }

            attempts += 1;

            if read_byte(&mut i2c, RESPONSE_0).is_ok_and(|response| response == RESPONSE_READY) {
                ready = true;
            } else {
                thread::sleep(Duration::from_millis(1));
            }
        }

        // Tell the sensor which channels to measure: 0 is the infrared
        // photodiode and 1 is the visible-light photodiode. Without CHAN_LIST a
        // FORCE command has nothing to measure and the output registers stay 0.
        param_set(&mut i2c, CHAN_LIST, 0b0000_0011)?;

        // Channel 0: infrared photodiode.
        param_set(&mut i2c, ADCCONFIG_0, 0b0000_0000)?;
        param_set(&mut i2c, ADCSENS_0, 0b1000_0000)?;
        param_set(&mut i2c, ADCPOST_0, 0b0000_0000)?;
        param_set(&mut i2c, MEASCONFIG_0, 0b0000_0000)?;

        // Channel 1: visible-light photodiode.
        param_set(&mut i2c, ADCCONFIG_1, 0b0000_1011)?;
        param_set(&mut i2c, ADCSENS_1, 0b1000_0000)?;
        param_set(&mut i2c, ADCPOST_1, 0b0000_0000)?;
        param_set(&mut i2c, MEASCONFIG_1, 0b0000_0000)?;

        Ok(Self { i2c })
    }

    /// Takes one measurement and returns the raw visible-light value.
    pub fn read_visible(&mut self) -> Result<u16, String> {
        read_visible_raw(&mut self.i2c)
    }
}

/// Connects to the sensor, configures its channels, and prints one reading.
pub fn show_sunlight() {
    println!("Connecting to sunlight sensor...");

    let mut sensor = match SunlightSensor::new() {
        Ok(sensor) => sensor,
        Err(error) => {
            println!("Could not use the sunlight sensor: {error}");

            // errno 121 is the kernel's way of saying that nothing answered at
            // the sensor's I2C address.
            if error.contains("Errno(121)") {
                println!();
                println!("Nothing answered at address 0x{ADDRESS:02X} on /dev/i2c-1.");
                println!("Check the wiring, then confirm the sensor is there with: i2cdetect -y 1");
            }

            return;
        }
    };

    println!("Detected Si1151.");

    match sensor.read_visible() {
        Ok(visible) => println!("Visible light (raw): {visible}"),
        Err(error) => println!("Could not read the sunlight sensor: {error}"),
    }
}
