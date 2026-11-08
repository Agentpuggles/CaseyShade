use std::io::{self, Write};
use std::thread::sleep;
use std::time::Duration;

mod analysis;
mod app;
mod data;
mod display;
mod graph;
mod logging;
mod network;
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

fn pause() {
    println!();
    println!("Press Enter to return to the menu.");

    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
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

    // Check whether CaseyShade was compiled for Linux.
    if !cfg!(target_os = "linux") {
        println!("Sorry, CaseyShade currently supports Linux only.");
        return;
    }

    println!("Linux is supported!");
    sleep(Duration::from_secs(1));

    let interval = logging::collection_interval();

    // The background collection and the dashboard are stopped when the
    // program exits.
    let mut logger: Option<logging::StreamHandle> = None;
    let mut server: Option<network::DashboardServer> = None;

    // Keep showing the menu until the user exits.
    loop {
        clear_screen();

        println!("=========================");
        println!("       CASEYSHADE");
        println!("=========================");
        println!("1. Read temperature, humidity and pressure");
        println!("2. Read sunlight (visible light)");
        println!("3. Read UV");
        println!("4. Live data graphs");

        if logger.is_some() {
            println!("5. Stop saving readings");
        } else {
            println!("5. Start saving readings (every {} s)", interval.as_secs());
        }

        println!("6. Daily statistics");
        println!("7. View data spreadsheet");
        println!("8. Check sensor status");
        println!("9. About CaseyShade and UV safety");

        if server.is_some() {
            println!("10. Stop the web dashboard");
        } else {
            println!("10. Start the web dashboard");
        }

        println!("11. Exit");

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
                pause();
            }
            "2" => {
                clear_screen();
                sensors::sunlight::show_sunlight();
                pause();
            }
            "3" => {
                clear_screen();
                sensors::uv::island();
                pause();
            }
            "4" => {
                graph::open_graph();
            }
            "5" => {
                clear_screen();

                if let Some(mut running) = logger.take() {
                    running.stop();
                    println!("Stopped saving readings.");
                } else {
                    match logging::start_stream(interval, true, None) {
                        Ok(stream) => {
                            logger = Some(stream);

                            println!(
                                "Saving a reading every {} s to {}.",
                                interval.as_secs(),
                                data::csv_path().display()
                            );
                        }
                        Err(error) => println!("Could not start saving readings: {error}"),
                    }
                }

                pause();
            }
            "6" => {
                clear_screen();
                display::show_statistics();
                pause();
            }
            "7" => {
                clear_screen();
                display::show_spreadsheet();
                pause();
            }
            "8" => {
                clear_screen();
                logging::show_sensor_status();
                pause();
            }
            "9" => {
                clear_screen();
                display::show_information();
                pause();
            }
            "10" => {
                clear_screen();

                if let Some(mut running) = server.take() {
                    running.stop();
                    println!("Stopped the web dashboard.");
                } else {
                    match network::start(network::port()) {
                        Ok(running) => server = Some(running),
                        Err(error) => println!("Could not start the web dashboard: {error}"),
                    }
                }

                pause();
            }
            "11" => {
                if let Some(mut running) = logger.take() {
                    running.stop();
                }

                if let Some(mut running) = server.take() {
                    running.stop();
                }

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
