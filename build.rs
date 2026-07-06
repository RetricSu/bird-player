use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Make cargo track changes to Cargo.toml and the current git ref.
    println!("cargo:rerun-if-changed=Cargo.toml");
    print_rerun_if_exists(".git/HEAD");
    print_rerun_if_exists(".git/packed-refs");
    if let Some(ref_path) = current_git_ref_path() {
        print_rerun_if_exists(&ref_path);
    }

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
            "pub const VERSION: &str = \"{version}\";\npub const GIT_HASH: &str = \"{git_hash}\";\n"
        ),
    )
    .unwrap();
}

fn print_rerun_if_exists(path: &str) {
    if Path::new(path).exists() {
        println!("cargo:rerun-if-changed={path}");
    }
}

fn current_git_ref_path() -> Option<String> {
    let head = fs::read_to_string(".git/HEAD").ok()?;
    let ref_name = head.strip_prefix("ref: ")?.trim();
    Some(format!(".git/{ref_name}"))
}
