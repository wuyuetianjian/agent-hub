use std::{env, fs, path::PathBuf, process::Command};

fn build_macos_overlay() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let native = manifest.join("../native/macos");
    println!("cargo:rerun-if-changed={}", native.display());
    println!("cargo:rerun-if-env-changed=MACOSX_DEPLOYMENT_TARGET");
    let target = env::var("TARGET").unwrap();
    let arch = match target.as_str() {
        "aarch64-apple-darwin" => "arm64",
        "x86_64-apple-darwin" => "x86_64",
        _ => panic!("unsupported macOS overlay target: {target}"),
    };
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let bridge = fs::read_to_string(native.join("bridge.js")).expect("read overlay bridge");
    let bridge_swift = out.join("BridgeSource.swift");
    fs::write(
        &bridge_swift,
        format!("let bridgeSource = #\"\"\"\n{bridge}\n\"\"\"#\n"),
    )
    .unwrap();
    let binaries = manifest.join("binaries");
    fs::create_dir_all(&binaries).unwrap();
    let binary = binaries.join(format!("agent-orbit-overlay-{target}"));
    let sdk = Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
        .expect("macOS overlay requires Xcode command line tools");
    assert!(sdk.status.success(), "xcrun failed to locate the macOS SDK");
    let sdk = String::from_utf8(sdk.stdout).unwrap();
    let deployment = env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| "11.0".into());
    let status = Command::new("xcrun")
        .args([
            "--sdk",
            "macosx",
            "swiftc",
            "-swift-version",
            "5",
            "-O",
            "-sdk",
            sdk.trim(),
            "-target",
            &format!("{arch}-apple-macosx{deployment}"),
            "-framework",
            "AppKit",
            "-framework",
            "WebKit",
        ])
        .arg(native.join("main.swift"))
        .arg(&bridge_swift)
        .arg("-o")
        .arg(&binary)
        .status()
        .expect("compile macOS overlay helper");
    assert!(status.success(), "macOS overlay Swift compilation failed");
    // Tauri's universal build runs Cargo for each real architecture. Once both
    // slices exist, supply the universal target name expected by its bundler.
    let arm = binaries.join("agent-orbit-overlay-aarch64-apple-darwin");
    let intel = binaries.join("agent-orbit-overlay-x86_64-apple-darwin");
    if arm.exists() && intel.exists() {
        let status = Command::new("xcrun")
            .args(["lipo", "-create"])
            .arg(&arm)
            .arg(&intel)
            .arg("-output")
            .arg(binaries.join("agent-orbit-overlay-universal-apple-darwin"))
            .status()
            .expect("combine macOS overlay architectures");
        assert!(
            status.success(),
            "macOS overlay universal binary creation failed"
        );
    }
    // The bundler signs externalBin and installs it beside the main executable.
    // Tauri's build helper also copies the sidecar to target/{profile} for dev.
}

fn main() {
    if env::var_os("CARGO_FEATURE_DESKTOP").is_some() {
        if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
            build_macos_overlay();
        }
        tauri_build::build()
    }
}
