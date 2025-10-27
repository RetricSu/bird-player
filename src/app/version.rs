// Include the version info module generated at build time
pub mod version_info {
    include!(concat!(env!("OUT_DIR"), "/version_info.rs"));

    // Return formatted version string with commit hash
    pub fn formatted_version() -> String {
        format!("Version {} ({})", VERSION, GIT_HASH)
    }
}
