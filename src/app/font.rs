/// Font setup and configuration for the application
///
/// Handles system font loading with CJK (Chinese, Japanese, Korean) support
use eframe::egui::{FontData, FontDefinitions, FontFamily};

/// Setup fonts for the application with CJK support
///
/// This function attempts to load system fonts with CJK support based on the current OS.
/// If no suitable CJK font is found, it falls back to a generic sans-serif font.
///
/// # Font Priority by OS:
/// - macOS: PingFang SC, Hiragino Sans GB, STSong, Heiti SC
/// - Windows: Microsoft YaHei, SimSun, SimHei, MS Gothic
/// - Linux: Noto Sans CJK TC
///
/// # Returns
/// A `FontDefinitions` instance configured with the best available system font
pub fn setup_fonts() -> FontDefinitions {
    // Create font definitions - start with defaults so we have fallbacks
    let mut fonts = FontDefinitions::default();

    // Try to find a system font with CJK support
    let source = font_kit::source::SystemSource::new();

    // Define font names to try based on OS for better CJK support
    let font_names: Vec<&str> = get_preferred_font_names();

    // Try to find one of the preferred fonts
    let mut found_font = false;
    for font_name in font_names {
        if try_load_font(&mut fonts, &source, font_name) {
            tracing::info!("Using system font '{}' for CJK support", font_name);
            found_font = true;
            break;
        }
    }

    // If we couldn't find any preferred fonts, try a generic sans-serif as backup
    if !found_font {
        load_fallback_font(&mut fonts, &source);
    }

    // Register the Phosphor icon font as a fallback so the icon constants in
    // `style::icons` render in the same labels/buttons as ordinary text.
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    fonts
}

/// Get preferred font names based on the current operating system
fn get_preferred_font_names() -> Vec<&'static str> {
    match std::env::consts::OS {
        "macos" => vec!["PingFang SC", "Hiragino Sans GB", "STSong", "Heiti SC"],
        "windows" => vec!["Microsoft YaHei", "SimSun", "SimHei", "MS Gothic"],
        "linux" => vec!["Noto Sans CJK TC"],
        _ => vec![], // Empty for other OSes - we'll use generic fallback
    }
}

/// Try to load a specific font by name
///
/// # Returns
/// `true` if the font was successfully loaded and registered, `false` otherwise
fn try_load_font(
    fonts: &mut FontDefinitions,
    source: &font_kit::source::SystemSource,
    font_name: &str,
) -> bool {
    // Get family by name
    if let Ok(family_handle) = source.select_family_by_name(font_name) {
        // For the first font in the family
        if let Some(font_handle) = family_handle.fonts().first() {
            if let Ok(font_data) = match font_handle {
                font_kit::handle::Handle::Memory { bytes, .. } => Ok(bytes.to_vec()),
                font_kit::handle::Handle::Path { path, .. } => std::fs::read(path),
            } {
                // Register the font with egui
                const SYSTEM_FONT_NAME: &str = "SystemCJKFont";
                fonts.font_data.insert(
                    SYSTEM_FONT_NAME.to_owned(),
                    FontData::from_owned(font_data).into(),
                );

                // Add as primary font for proportional text (at the beginning)
                fonts
                    .families
                    .get_mut(&FontFamily::Proportional)
                    .unwrap()
                    .insert(0, SYSTEM_FONT_NAME.to_owned());

                // Also add to monospace as a fallback
                fonts
                    .families
                    .get_mut(&FontFamily::Monospace)
                    .unwrap()
                    .push(SYSTEM_FONT_NAME.to_owned());

                return true;
            }
        }
    }

    false
}

/// Load a fallback font (generic sans-serif) when no preferred font is available
fn load_fallback_font(fonts: &mut FontDefinitions, source: &font_kit::source::SystemSource) {
    if let Ok(font_handle) = source.select_best_match(
        &[font_kit::family_name::FamilyName::SansSerif],
        &font_kit::properties::Properties::new(),
    ) {
        if let Ok(font_data) = match font_handle {
            font_kit::handle::Handle::Memory { bytes, .. } => Ok(bytes.to_vec()),
            font_kit::handle::Handle::Path { path, .. } => std::fs::read(&path),
        } {
            const SYSTEM_FONT_NAME: &str = "SystemFont";
            fonts.font_data.insert(
                SYSTEM_FONT_NAME.to_owned(),
                FontData::from_owned(font_data).into(),
            );

            // Add as primary font
            fonts
                .families
                .get_mut(&FontFamily::Proportional)
                .unwrap()
                .insert(0, SYSTEM_FONT_NAME.to_owned());

            tracing::info!("Using generic system font for text");
        } else {
            tracing::warn!("Could not load system font data, using defaults");
        }
    } else {
        tracing::warn!("Could not find suitable system font, using defaults");
    }
}
