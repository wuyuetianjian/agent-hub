#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("bridge") {
        let kind = args.get(2).map(String::as_str).unwrap_or("permission");
        let runtime = tokio::runtime::Runtime::new().expect("Cannot create hook runtime");
        if runtime.block_on(agent_orbit::bridge::helper(kind)).is_err() {
            std::process::exit(1);
        }
        return;
    }
    #[cfg(feature = "desktop")]
    agent_orbit::desktop::run();
    #[cfg(not(feature = "desktop"))]
    eprintln!("Build with the desktop feature to launch the window");
}
