use std::thread::sleep;

fn main() {
    println!("Welcome to Casey Shade!");
    sleep(core::time::Duration::from_millis(2000));

    println!("Let's check your operating system:\n");

    let os = os_info::get();

    if os.os_type() == os_info::Type::Windows {
        println!("Windows operating system is not supported.");
        std::process::exit(1);
    }
}