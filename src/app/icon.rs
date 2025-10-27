use eframe::egui;
use image;

/// Load the app icon from multiple possible locations for both development and bundled app scenarios
pub fn get_app_icon() -> Option<egui::IconData> {
    // Try different potential paths for both development and bundled app
    let icon_paths = [
        "./assets/icons/icon.png",             // Development path
        "../assets/icons/icon.png",            // Relative to release dir
        "../Resources/assets/icons/icon.png",  // Relative to app bundle
        "./assets/icons/icon.icns",            // Development path icns
        "../assets/icons/icon.icns",           // Relative to release dir icns
        "../Resources/assets/icons/icon.icns", // Relative to app bundle icns
    ];

    for path in icon_paths {
        if let Ok(icon) = image::open(path) {
            let icon = icon.to_rgba8();
            let (width, height) = icon.dimensions();
            return Some(egui::IconData {
                rgba: icon.into_raw(),
                width,
                height,
            });
        }
    }

    // If all paths failed, log it but continue without an icon
    tracing::warn!("Could not load app icon from any path");
    None
}
