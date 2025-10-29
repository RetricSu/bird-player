use eframe::egui;
use image;
use std::env;
use std::path::{Path, PathBuf};

/// Load the app icon from multiple possible locations for both development and bundled app scenarios
pub fn get_app_icon() -> Option<egui::IconData> {
    let mut icon_paths: Vec<PathBuf> = vec![
        PathBuf::from("./assets/icons/icon.png"),
        PathBuf::from("./assets/icons/icon.icns"),
        PathBuf::from("../assets/icons/icon.png"),
        PathBuf::from("../assets/icons/icon.icns"),
    ];

    if let Ok(exe_path) = env::current_exe() {
        // When running inside a macOS bundle the executable lives in Contents/MacOS
        if let Some(contents_dir) = exe_path.parent().and_then(|macos| macos.parent()) {
            let resources_dir = contents_dir.join("Resources");
            icon_paths.push(resources_dir.join("icon.icns"));
            icon_paths.push(resources_dir.join("icon.png"));
            icon_paths.push(resources_dir.join("assets/icons/icon.icns"));
            icon_paths.push(resources_dir.join("assets/icons/icon.png"));
        }
    }

    icon_paths
        .into_iter()
        .filter_map(|path| load_icon(&path))
        .next()
        .or_else(|| {
            tracing::warn!("Could not load app icon from any path");
            None
        })
}

fn load_icon(path: &Path) -> Option<egui::IconData> {
    let icon = image::open(path).ok()?;
    let icon = icon.to_rgba8();
    let (width, height) = icon.dimensions();

    Some(egui::IconData {
        rgba: icon.into_raw(),
        width,
        height,
    })
}
