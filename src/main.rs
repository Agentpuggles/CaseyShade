
use std::io::{self, Write};
use std::thread::sleep;
use std::time::Duration;

mod sensors;

fn clear_screen() {
    print!("\x1B[2J\x1B[H");
    io::stdout().flush().unwrap();
}

fn loading(message: &str) {
    print!("{message}");
    io::stdout().flush().unwrap();

    for _ in 0..3 {
        sleep(Duration::from_millis(400));
        print!(".");
        io::stdout().flush().unwrap();
    }

    println!();
}

fn main() {
    clear_screen();

    println!("Welcome to Casey Shade!");
    println!();

    loading("Checking operating system");

    // Get information about the operating system.
    let operating_system = os_info::get();
    let operating_system_name = operating_system.to_string();

    println!("Detected OS: {operating_system_name}");

    // Check whether the operating system name contains "Linux".
    if !operating_system_name.contains("Linux") {
        println!("Sorry, CaseyShade currently supports Linux only.");
        return;
    }

    println!("Linux is supported!");
    sleep(Duration::from_secs(1));

    // Keep showing the menu until the user exits.
    loop {
        clear_screen();

        println!("=========================");
        println!("       CASEYSHADE");
        println!("=========================");
        println!("1. Read temperature");
        println!("2. Read sunlight and UV");
        println!("3. Exit");
        println!();

        print!("Choose an option: ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();

        if io::stdin().read_line(&mut choice).is_err() {
            println!("Could not read your input.");
            break;
        }


        match choice.trim() {
            "1" => {
                clear_screen();
                sensors::temperature::show_temperature();

                println!();
                println!("Press Enter to return to the menu.");

                let mut input = String::new();
                let _ = io::stdin().read_line(&mut input);
            }
            "2" => {
                clear_screen();
                sensors::sunlight::show_sunlight();

                println!();
                println!("Press Enter to return to the menu.");

                let mut input = String::new();
                let _ = io::stdin().read_line(&mut input);
            }
            "3" => {
                println!("Goodbye!");
                break;
            }
            _ => {
                println!("Invalid option. Try again.");
                sleep(Duration::from_secs(1));
            }
        }

    }
}
