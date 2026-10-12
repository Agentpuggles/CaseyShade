pub fn operating_system() {
    let os = os_info::get();
    println!("OS: {}", os);
    if os.contains()("Windows") {
        return Err("Windows operating system is not supported.".to_string());
    }

}