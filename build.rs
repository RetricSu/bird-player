use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Make cargo track changes to Cargo.toml
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Configure Windows to use the windows subsystem (no console window)
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");
        println!("cargo:rustc-link-arg=/ENTRY:mainCRTStartup");
    }

    // Get version from Cargo.toml
    let version = env!("CARGO_PKG_VERSION");

    // Try to get git commit hash
    let git_hash = match Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "unknown".to_string(),
    };

    // Create a version info module
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("version_info.rs");

    fs::write(
        &dest_path,
        format!(
            "pub const VERSION: &str = \"{}\";\npub const GIT_HASH: &str = \"{}\";\n",
            version, git_hash
        ),
    )
    .unwrap();
}
