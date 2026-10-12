pub mod hardware;

use std::thread::sleep;
use crate::hardware::operating_system;

fn main() {
    println!("Welcome to Casey Shade!");
    sleep(core::time::Duration::from_millis(2000));
    println!("Lets check your operating system:\n");
    operating_system();
}