use embedded_hal::i2c::I2c;
use linux_embedded_hal::I2cdev;
use std::thread;
use std::time::Duration;

// ---- Hardware constants (from the Si115x datasheet) ----
const ADDRESS: u8 = 0x53; // Si1151 default I2C address

// Registers
const PART_ID:    u8 = 0x00; // Device identification register
const HOSTIN_0:   u8 = 0x0A; // Input register
const COMMAND:    u8 = 0x0B; // Writing a command here RESET_SW FORCE PARAM_SET tells it to send something
const RESPONSE_0: u8 = 0x11; // Status register
const HOSTOUT_2:  u8 = 0x15; // visible channel, high byte
const HOSTOUT_3:  u8 = 0x16; // visible channel, low byte

// Command codes
const RESET_SW:  u8 = 0x01; // Reboot the chip
const FORCE:     u8 = 0x11; // Take a measurement right now command
const PARAM_SET: u8 = 0x80; // Adding parameters to an address

// Channel 0 infrared photodiode
const ADCCONFIG_0:  u8 = 0x02;
const ADCSENS_0:    u8 = 0x03; // sensitivity
const ADCPOST_0:    u8 = 0x04; // output post processing
const MEASCONFIG_0: u8 = 0x05; // When and how often to measure

// Visible light values
const ADCCONFIG_1:  u8 = 0x06;
const ADCSENS_1:    u8 = 0x07;
const ADCPOST_1:    u8 = 0x08;
const MEASCONFIG_1: u8 = 0x09;

/// I2C reads in these steps, write the register number
/// "I want register x" then we read the reply to it
/// write_read does both
fn read_byte(i2c: &mut I2cdev, reg: u8) -> u8 {
    // Create a small buffer to hold the reply from the sensor.
    // [0u8; 1] means an array containing one byte, initially set to 0.
    let mut buf = [0u8; 1];
    // Talk to the sensor using I2C.
    // First, send the register number we want to read (`reg`).
    // Then, receive one byte of data and store it in `buf`.
    // `&mut buf` lets the I2C function write the received data into our buffer.
    // `.expect(...)` stops the program and displays an error if the read fails.
    i2c.write_read(ADDRESS, &[reg], &mut buf).expect("I2C read failed");
    // Return the byte we received from the sensor.
    // `buf[0]` means the first (and only) item in the array.
    buf[0]
}


/// Writes one byte of data to a sensor register over I2C.
///
/// `i2c` is our connection to the sensor.
/// `reg` is the register number we want to write to.
/// `value` is the data we want to put into that register.
/// There is no `-> u8` because this function does not return a value.
fn write_byte(i2c: &mut I2cdev, reg: u8, value: u8) {
    // Send data to the sensor using I2C.
    // `ADDRESS` tells Rust which sensor we want to communicate with.
    // `&[reg, value]` creates an array containing two bytes:
    // 1. The register number we want to write to.
    // 2. The value we want to store in that register.
    // `.write(...)` sends these bytes to the sensor.
    // `.expect(...)` stops the program and displays an error if the write fails.
    i2c.write(ADDRESS, &[reg, value])
        .expect("I2C write failed");
}




/// Wait until the sensor's response counter changes to the expected value.
/// This helps us check whether the sensor has processed a command.
fn wait_for_ack(i2c: &mut I2cdev, previous_response: u8) {
    // Calculate the next response counter value we expect from the sensor.
    // `wrapping_add(1)` adds 1 to the previous response number.
    // `& 0x0F` keeps only the lowest 4 bits, giving a value from 0 to 15.
    // The counter wraps back to 0 after reaching 15 because it uses 4 bits.
    let expected = previous_response.wrapping_add(1) & 0x0F;

    // Keep repeating the following code until the sensor responds as expected.
    loop {
        // Read one byte from the sensor's response register.
        // `RESPONSE_0` is the register address (0x11 in hexadecimal).
        // `read_byte` sends that register address and reads the sensor's reply.
        // `& 0x0F` extracts the lowest 4 bits, which we expect to contain the counter.
        // `== expected` checks whether the counter matches the value we calculated.
        if (read_byte(i2c, RESPONSE_0) & 0x0F) == expected {
            // The response counter matches the expected value!
            // `return` exits this function so the rest of the program can continue.
            return;
        }

        // The sensor has not reported the expected counter value yet.
        // Pause for 200 microseconds (0.2 milliseconds) before checking again.
        // This gives the sensor time to process the command between checks.
        thread::sleep(Duration::from_micros(200));
    }
}



/// Tell the sensor to store `value` in the parameter identified by `location`.
fn param_set(i2c: &mut I2cdev, location: u8, value: u8) {
    // Read and save the sensor's current response counter.
    // We'll use this value later to check whether the sensor has processed our command.
    let before = read_byte(i2c, RESPONSE_0);

    // Write the value we want to store into the sensor's HOSTIN_0 register.
    // `i2c` is our connection to the sensor.
    // `HOSTIN_0` is the register where we place the value before sending the command.
    // `value` is the data we want the sensor to store in the parameter.
    write_byte(i2c, HOSTIN_0, value);

    // Tell the sensor to perform the PARAM_SET operation on the chosen parameter.
    // `location` identifies which parameter we want to change.
    // `|` is the bitwise OR operator, which combines the bits in `location` and `PARAM_SET`.
    // `COMMAND` is the register where we send the command to the sensor.
    write_byte(i2c, COMMAND, location | PARAM_SET);

    // Wait for the sensor's response counter to advance to the expected value.
    // We pass `before` so `wait_for_ack` knows which counter value we started with.
    // This lets the program check that the command has been processed before continuing.
    wait_for_ack(i2c, before);
}



/// Send a command to the sensor, such as RESET_SW or FORCE.
fn send_command(i2c: &mut I2cdev, code: u8) {
    // Read and save the sensor's current response counter.
    // We'll compare it with the counter after sending the command
    // to check whether the sensor has processed our command.
    let before = read_byte(i2c, RESPONSE_0);

    // Send the command to the sensor's COMMAND register.
    // `COMMAND` identifies the register where commands are written.
    // `code` is the command number we want the sensor to execute.
    // `write_byte` sends the register address followed by the command value.
    write_byte(i2c, COMMAND, code);

    // Wait for the sensor's response counter to reach the expected value.
    // We pass `before` so the function knows the counter value from before the command.
    // This prevents our program from continuing until the expected response is detected.
    wait_for_ack(i2c, before);
}



// ---------- Public API ----------
// These are the functions that other parts of our program can use.

/// Read the visible-light value from the sensor.
/// The result is a raw 16-bit number, not a measurement in lux.
pub fn read_visible(i2c: &mut I2cdev) -> u16 {
    // Ask the sensor to take one measurement.
    // FORCE tells the sensor to start a measurement immediately.
    send_command(i2c, FORCE);

    // Read the high byte of the measurement (the most significant 8 bits).
    // HOSTOUT_2 is the register containing the high byte of our reading.
    let hi = read_byte(i2c, HOSTOUT_2);

    // Read the low byte of the measurement (the least significant 8 bits).
    // HOSTOUT_3 is the register containing the low byte of our reading.
    let lo = read_byte(i2c, HOSTOUT_3);

    // Combine the two 8-bit values into one 16-bit value.
    // `hi as u16` converts the high byte into a 16-bit number.
    // `<< 8` shifts its bits 8 places to the left.
    // `lo as u16` converts the low byte into a 16-bit number.
    // `|` combines the high and low bits into the final reading.
    ((hi as u16) << 8) | (lo as u16)
}

/// Connect to the sensor, configure its channels, and print one reading.
pub fn show_sunlight() {
    // Tell the user that the program is starting to connect to the sensor.
    println!("Connecting to sunlight sensor...");

    // Open the Raspberry Pi's I2C bus.
    // `/dev/i2c-1` is the device file for I2C bus 1 on our Raspberry Pi.
    // `let mut` creates a variable we can change and use for I2C communication.
    // `match` lets us handle both a successful connection and an error.
    let mut i2c = match I2cdev::new("/dev/i2c-1") {
        // If opening the I2C bus succeeds, save the device connection in `dev`.
        Ok(dev) => dev,

        // If opening the I2C bus fails, `e` contains the error information.
        Err(e) => {
            // Print the error so we know why the connection failed.
            println!("Could not open I2C: {e:?}");

            // Exit this function because we cannot communicate with the sensor.
            return;
        }
    };

    // 1. Check the sensor's identity before trying to configure it.
    // PART_ID is the register containing the sensor's part identification number.
    let id = read_byte(&mut i2c, PART_ID);

    // The SI1151 is expected to report the part ID 0x51.
    // If the ID is different, stop because we may not be talking to the expected sensor.
    if id != 0x51 {
        // `{:02X}` formats the ID as two hexadecimal digits.
        // This helps us see the actual ID returned by the sensor.
        println!("Unexpected part ID 0x{id:02X} (expected 0x51)");

        // Exit because the sensor identity check failed.
        return;
    }

    // The part ID matched, so tell the user that the sensor was detected.
    println!("Detected Si1151.");

    // 2. Reset the sensor using its software-reset command.
    // RESET_SW tells the sensor to return to its reset state.
    write_byte(&mut i2c, COMMAND, RESET_SW);

    // Keep checking RESPONSE_0 until the response register equals 0x2F.
    // The 1-millisecond pause gives the sensor time to reset between checks.
    // This assumes 0x2F is the correct reset-completion value for this device.
    while read_byte(&mut i2c, RESPONSE_0) != 0x2F {
        // Pause for 1 millisecond before reading the register again.
        thread::sleep(Duration::from_millis(1));
    }

    // 3. Configure channel 0 for infrared (IR) measurements.
    // ADCCONFIG_0 selects the detector and measurement configuration for channel 0.
    // 0b0000_0000 is a binary value with all eight bits set to 0.
    // This setting is intended to select the small IR photodiode.
    param_set(&mut i2c, ADCCONFIG_0, 0b0000_0000);

    // ADCSENS_0 configures the sensitivity and signal range for channel 0.
    // 0b1000_0000 sets the highest bit to 1 and the other bits to 0.
    // This setting is intended to select the high signal range.
    param_set(&mut i2c, ADCSENS_0, 0b1000_0000);

    // ADCPOST_0 configures the post-processing options for channel 0.
    // Here, we write 0 to use the intended default post-processing settings.
    param_set(&mut i2c, ADCPOST_0, 0b0000_0000);

    // MEASCONFIG_0 configures additional measurement behaviour for channel 0.
    // Here, we write 0 to select the intended default configuration.
    param_set(&mut i2c, MEASCONFIG_0, 0b0000_0000);

    // 4. Configure channel 1 for visible-light measurements.
    // ADCCONFIG_1 selects the detector and measurement configuration for channel 1.
    // 0b0000_1011 sets bits 0, 1, and 3 to 1.
    // This value is intended to select the visible-light photodiode.
    param_set(&mut i2c, ADCCONFIG_1, 0b0000_1011);

    // ADCSENS_1 configures the sensitivity and signal range for channel 1.
    // This value is intended to select the high signal range.
    param_set(&mut i2c, ADCSENS_1, 0b1000_0000);

    // ADCPOST_1 configures the post-processing options for channel 1.
    // Write 0 to select the intended default post-processing settings.
    param_set(&mut i2c, ADCPOST_1, 0b0000_0000);

    // MEASCONFIG_1 configures additional measurement behaviour for channel 1.
    // Write 0 to select the intended default configuration.
    param_set(&mut i2c, MEASCONFIG_1, 0b0000_0000);

    // 5. Take a visible-light measurement.
    // Pass the existing I2C connection into read_visible.
    // The returned value is stored in the `visible` variable.
    let visible = read_visible(&mut i2c);

    // Print the raw measurement value to the terminal.
    // `{visible}` inserts the value of the variable into the message.
    println!("Visible light (raw): {visible}");
}
