use std::thread::sleep;

fn main() {
    println!("Welcome to Casey Shade!");
    sleep(core::time::Duration::from_secs(1));

    println!("Let's check your operating system:");
    sleep(core::time::Duration::from_secs(1));
    println!("...");
    sleep(core::time::Duration::from_secs(1));
    println!("...");
    sleep(core::time::Duration::from_secs(1));
    println!("...");

    let os = os_info::get();

    if os.os_type() == os_info::Type::Windows {
        println!("Windows operating system is not supported.");
        std::process::exit(1);
    }
}