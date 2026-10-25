#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("codex-dispatch") {
        let result = if let Some(path) = args.get(2) {
            tokio::runtime::Runtime::new()
                .expect("Cannot create hook runtime")
                .block_on(agent_orbit::codex_dispatcher::helper(std::path::Path::new(
                    path,
                )))
        } else {
            serde_json::json!({})
        };
        println!("{result}");
        return;
    }
    if args.get(1).map(String::as_str) == Some("codex-hook") {
        let result = if let Some(path) = args.get(2) {
            tokio::runtime::Runtime::new()
                .expect("Cannot create hook runtime")
                .block_on(agent_orbit::codex_hook::run_helper(std::path::Path::new(
                    path,
                )))
        } else {
            serde_json::json!({})
        };
        println!("{result}");
        return;
    }
    if args.get(1).map(String::as_str) == Some("install-codex-hook") {
        let result = match (args.get(2), args.get(3)) {
            (Some(config), Some(target)) => agent_orbit::codex_hook::install(
                std::path::Path::new(config),
                std::path::Path::new(target),
            ),
            _ => Err(anyhow::anyhow!(
                "Expected generated configuration and target hooks.json paths"
            )),
        };
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        println!("Hook configuration installed. Review and trust it in the original Codex client.");
        return;
    }
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
