use embedded_hal::i2c::I2c;
use linux_embedded_hal::I2cdev;
use std::thread;
use std::time::Duration;

// ---- Hardware constants (from the Si115x datasheet) ----
const ADDRESS: u8 = 0x53; // Si1151 default I2C address

// Registers
const PART_ID:    u8 = 0x00;
const HOSTIN_0:   u8 = 0x0A;
const COMMAND:    u8 = 0x0B;
const RESPONSE_0: u8 = 0x11;
const HOSTOUT_2:  u8 = 0x15; // visible channel, high byte
const HOSTOUT_3:  u8 = 0x16; // visible channel, low byte

// Command codes
const RESET_SW:  u8 = 0x01;
const FORCE:     u8 = 0x11;
const PARAM_SET: u8 = 0x80;

// Parameter addresses (used to configure each measurement channel)
const ADCCONFIG_0:  u8 = 0x02;
const ADCSENS_0:    u8 = 0x03;
const ADCPOST_0:    u8 = 0x04;
const MEASCONFIG_0: u8 = 0x05;
const ADCCONFIG_1:  u8 = 0x06;
const ADCSENS_1:    u8 = 0x07;
const ADCPOST_1:    u8 = 0x08;
const MEASCONFIG_1: u8 = 0x09;

// ---------- low level helpers ----------

/// Read one byte from a register.
fn read_byte(i2c: &mut I2cdev, reg: u8) -> u8 {
    let mut buf = [0u8; 1];
    i2c.write_read(ADDRESS, &[reg], &mut buf).expect("I2C read failed");
    buf[0]
}

/// Write one byte to a register.
fn write_byte(i2c: &mut I2cdev, reg: u8, value: u8) {
    i2c.write(ADDRESS, &[reg, value]).expect("I2C write failed");
}

/// Wait until the sensor increments its 4-bit response counter.
/// This is how the chip tells us it has finished a command.
fn wait_for_ack(i2c: &mut I2cdev, previous_response: u8) {
    let expected = previous_response.wrapping_add(1) & 0x0F;
    loop {
        if (read_byte(i2c, RESPONSE_0) & 0x0F) == expected {
            return;
        }
        thread::sleep(Duration::from_micros(200));
    }
}

/// Tell the sensor to store `value` in parameter `location`.
fn param_set(i2c: &mut I2cdev, location: u8, value: u8) {
    let before = read_byte(i2c, RESPONSE_0);
    write_byte(i2c, HOSTIN_0, value);                 // put the value in the mailbox
    write_byte(i2c, COMMAND, location | PARAM_SET);   // then ask the chip to take it
    wait_for_ack(i2c, before);
}

/// Send a command (e.g. RESET_SW, FORCE).
fn send_command(i2c: &mut I2cdev, code: u8) {
    let before = read_byte(i2c, RESPONSE_0);
    write_byte(i2c, COMMAND, code);
    wait_for_ack(i2c, before);
}

// ---------- public API ----------

/// Read the visible-light value (raw 16-bit counts).
pub fn read_visible(i2c: &mut I2cdev) -> u16 {
    send_command(i2c, FORCE); // ask the sensor for one measurement
    let hi = read_byte(i2c, HOSTOUT_2);
    let lo = read_byte(i2c, HOSTOUT_3);
    ((hi as u16) << 8) | (lo as u16)
}

/// Configure the sensor, then print one visible-light reading.
pub fn show_sunlight() {
    println!("Connecting to sunlight sensor...");

    // Open /dev/i2c-1
    let mut i2c = match I2cdev::new("/dev/i2c-1") {
        Ok(dev) => dev,
        Err(e) => {
            println!("Could not open I2C: {e}");
            return;
        }
    };

    // 1. Check that we are really talking to an Si1151 (part ID 0x51)
    let id = read_byte(&mut i2c, PART_ID);
    if id != 0x51 {
        println!("Unexpected part ID 0x{id:02X} (expected 0x51)");
        return;
    }
    println!("Detected Si1151.");

    // 2. Software reset, then wait for it to finish (fixed value 0x2F)
    write_byte(&mut i2c, COMMAND, RESET_SW);
    while read_byte(&mut i2c, RESPONSE_0) != 0x2F {
        thread::sleep(Duration::from_millis(1));
    }

    // 3. Configure channel 0 (IR) — not read here, but matches the C++ library
    param_set(&mut i2c, ADCCONFIG_0,  0b0000_0000); // small IR photodiode
    param_set(&mut i2c, ADCSENS_0,    0b1000_0000); // high signal range
    param_set(&mut i2c, ADCPOST_0,    0b0000_0000);
    param_set(&mut i2c, MEASCONFIG_0, 0b0000_0000);

    // 4. Configure channel 1 (Visible)
    param_set(&mut i2c, ADCCONFIG_1,  0b0000_1011); // visible photodiode
    param_set(&mut i2c, ADCSENS_1,    0b1000_0000); // high signal range
    param_set(&mut i2c, ADCPOST_1,    0b0000_0000);
    param_set(&mut i2c, MEASCONFIG_1, 0b0000_0000);

    // 5. Take a reading
    let visible = read_visible(&mut i2c);
    println!("Visible light (raw): {visible}");
}