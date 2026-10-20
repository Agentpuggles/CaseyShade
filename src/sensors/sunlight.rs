use embedded_hal::i2c::I2c;
use linux_embedded_hal::I2cdev;

const SI115X_ADDRESS: u8 = 0x53;
const PART_ID_REGISTER: u8 = 0x00;

pub fn show_sunlight() {
    println!("Connecting to sunlight sensor...");

    let mut i2c = match I2cdev::new("/dev/i2c-1") {
        Ok(device) => device,
        Err(error) => {
            println!("Could not connect to I2C: {error}");
            return;
        }
    };

    let mut part_id = [0u8; 1];

    match i2c.write_read(
        SI115X_ADDRESS,
        &[PART_ID_REGISTER],
        &mut part_id,
    ) {
        Ok(()) => {
            println!("Sensor responded!");
            println!("Part ID: 0x{:02X}", part_id[0]);

            match part_id[0] {
                0x51 => println!("Detected SI1151."),
                0x52 => println!("Detected SI1152."),
                0x53 => println!("Detected SI1153."),
                other => println!("Unrecognised part ID: 0x{other:02X}"),
            }
        }
        Err(error) => {
            println!("Could not read sensor ID: {error}");
        }
    }
}