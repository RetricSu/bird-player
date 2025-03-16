use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

// Supported languages
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    #[default]
    English,
    Chinese,
}

impl Language {
    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Chinese => "zh",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Language::English => "En",
            Language::Chinese => "中文",
        }
    }

    pub fn all() -> Vec<Language> {
        vec![Language::English, Language::Chinese]
    }
}

// Define a type for translations
type Translations = HashMap<String, String>;

// Global state for i18n
lazy_static! {
    static ref CURRENT_LANGUAGE: RwLock<Language> = RwLock::new(Language::default());
    static ref TRANSLATIONS: RwLock<HashMap<Language, Translations>> = RwLock::new(HashMap::new());
}

// Initialize translations
pub fn init() {
    let mut translations = HashMap::new();

    // English translations
    let mut en = HashMap::new();
    // File menu
    en.insert("file".to_string(), "File".to_string());
    en.insert("open".to_string(), "Open".to_string());
    en.insert("settings".to_string(), "Settings".to_string());
    en.insert("exit".to_string(), "Exit".to_string());

    // Playback menu
    en.insert("playback".to_string(), "Playback".to_string());
    en.insert("play_pause".to_string(), "Play/Pause".to_string());
    en.insert("previous".to_string(), "Previous".to_string());
    en.insert("next".to_string(), "Next".to_string());
    en.insert("play_mode".to_string(), "Play Mode: {}".to_string());

    // Help menu
    en.insert("help".to_string(), "Help".to_string());
    en.insert("about".to_string(), "About".to_string());

    // Player component
    en.insert("song".to_string(), "Song: ".to_string());
    en.insert("artist".to_string(), "Artist: ".to_string());
    en.insert("playlist".to_string(), "Playlist: ".to_string());
    en.insert("no_track".to_string(), "No track selected".to_string());
    en.insert(
        "select_track".to_string(),
        "Select a track from the playlist to play".to_string(),
    );
    en.insert(
        "add_tracks".to_string(),
        "Add tracks to your playlist to start playing".to_string(),
    );
    en.insert(
        "create_playlist".to_string(),
        "Create a playlist to start playing music".to_string(),
    );
    en.insert("remove_song".to_string(), "Remove Song".to_string());
    en.insert("mini".to_string(), "Mini".to_string());
    en.insert("playlist_btn".to_string(), "Playlist".to_string());
    en.insert("lyrics".to_string(), "Lyrics".to_string());

    // Library component
    en.insert("music_files".to_string(), "Music Library".to_string());
    en.insert("expand_all".to_string(), "Expand all folders".to_string());
    en.insert(
        "collapse_all".to_string(),
        "Collapse all folders".to_string(),
    );
    en.insert("resync_all".to_string(), "Re-sync all folders".to_string());
    en.insert(
        "add_music_folder".to_string(),
        "Add music folder".to_string(),
    );
    en.insert("unknown_title".to_string(), "Unknown Title".to_string());
    en.insert("unknown_track".to_string(), "Unknown Track".to_string());
    en.insert("add_to_playlist".to_string(), "Add to playlist".to_string());
    en.insert(
        "add_all_to_playlist".to_string(),
        "Add all to playlist".to_string(),
    );
    en.insert(
        "remove_from_library".to_string(),
        "Remove from library".to_string(),
    );

    // Playlist tabs component
    en.insert("rename".to_string(), "Rename".to_string());
    en.insert("delete".to_string(), "Delete".to_string());
    en.insert("new_playlist".to_string(), "New Playlist".to_string());
    en.insert("enter_name".to_string(), "Enter name...".to_string());

    // Playlist table component
    en.insert("column_number".to_string(), "#".to_string());
    en.insert("column_title".to_string(), "Title".to_string());
    en.insert("column_artist".to_string(), "Artist".to_string());
    en.insert("column_album".to_string(), "Album".to_string());
    en.insert("column_genre".to_string(), "Genre".to_string());
    en.insert("edit_title".to_string(), "Edit title".to_string());
    en.insert("edit_artist".to_string(), "Edit artist".to_string());
    en.insert("edit_album".to_string(), "Edit album".to_string());
    en.insert("edit_genre".to_string(), "Edit genre".to_string());
    en.insert(
        "remove_from_playlist".to_string(),
        "Remove from playlist".to_string(),
    );
    en.insert("unknown_title".to_string(), "unknown title".to_string());
    en.insert("unknown_artist".to_string(), "unknown artist".to_string());
    en.insert("unknown_album".to_string(), "unknown album".to_string());
    en.insert("unknown_genre".to_string(), "unknown genre".to_string());

    // Chinese translations
    let mut zh = HashMap::new();
    // File menu
    zh.insert("file".to_string(), "文件".to_string());
    zh.insert("open".to_string(), "打开".to_string());
    zh.insert("settings".to_string(), "设置".to_string());
    zh.insert("exit".to_string(), "退出".to_string());

    // Playback menu
    zh.insert("playback".to_string(), "播放".to_string());
    zh.insert("play_pause".to_string(), "播放/暂停".to_string());
    zh.insert("previous".to_string(), "上一首".to_string());
    zh.insert("next".to_string(), "下一首".to_string());
    zh.insert("play_mode".to_string(), "播放模式: {}".to_string());

    // Help menu
    zh.insert("help".to_string(), "帮助".to_string());
    zh.insert("about".to_string(), "关于".to_string());

    // Player component
    zh.insert("song".to_string(), "歌曲：".to_string());
    zh.insert("artist".to_string(), "艺术家：".to_string());
    zh.insert("playlist".to_string(), "播放列表：".to_string());
    zh.insert("no_track".to_string(), "未选择歌曲".to_string());
    zh.insert(
        "select_track".to_string(),
        "从播放列表中选择一首歌曲播放".to_string(),
    );
    zh.insert(
        "add_tracks".to_string(),
        "添加歌曲到播放列表开始播放".to_string(),
    );
    zh.insert(
        "create_playlist".to_string(),
        "创建播放列表开始播放音乐".to_string(),
    );
    zh.insert("remove_song".to_string(), "移除歌曲".to_string());
    zh.insert("mini".to_string(), "迷你".to_string());
    zh.insert("playlist_btn".to_string(), "列表".to_string());
    zh.insert("lyrics".to_string(), "歌词".to_string());

    // Library component
    zh.insert("music_files".to_string(), "音乐库".to_string());
    zh.insert("expand_all".to_string(), "展开所有文件夹".to_string());
    zh.insert("collapse_all".to_string(), "折叠所有文件夹".to_string());
    zh.insert("resync_all".to_string(), "重新同步所有文件夹".to_string());
    zh.insert("add_music_folder".to_string(), "添加音乐文件夹".to_string());
    zh.insert("unknown_title".to_string(), "未知标题".to_string());
    zh.insert("unknown_track".to_string(), "未知曲目".to_string());
    zh.insert("add_to_playlist".to_string(), "添加到播放列表".to_string());
    zh.insert(
        "add_all_to_playlist".to_string(),
        "全部添加到播放列表".to_string(),
    );
    zh.insert("remove_from_library".to_string(), "从库中移除".to_string());

    // Playlist tabs component
    zh.insert("rename".to_string(), "重命名".to_string());
    zh.insert("delete".to_string(), "删除".to_string());
    zh.insert("new_playlist".to_string(), "新播放列表".to_string());
    zh.insert("enter_name".to_string(), "输入名称...".to_string());

    // Playlist table component
    zh.insert("column_number".to_string(), "#".to_string());
    zh.insert("column_title".to_string(), "标题".to_string());
    zh.insert("column_artist".to_string(), "艺术家".to_string());
    zh.insert("column_album".to_string(), "专辑".to_string());
    zh.insert("column_genre".to_string(), "类型".to_string());
    zh.insert("edit_title".to_string(), "编辑标题".to_string());
    zh.insert("edit_artist".to_string(), "编辑艺术家".to_string());
    zh.insert("edit_album".to_string(), "编辑专辑".to_string());
    zh.insert("edit_genre".to_string(), "编辑类型".to_string());
    zh.insert(
        "remove_from_playlist".to_string(),
        "从播放列表中移除".to_string(),
    );
    zh.insert("unknown_title".to_string(), "未知标题".to_string());
    zh.insert("unknown_artist".to_string(), "未知艺术家".to_string());
    zh.insert("unknown_album".to_string(), "未知专辑".to_string());
    zh.insert("unknown_genre".to_string(), "未知类型".to_string());

    // Add about window translations
    init_about_translations(&mut en, &mut zh);

    // Add translations to the global map
    translations.insert(Language::English, en);
    translations.insert(Language::Chinese, zh);

    // Store translations
    let mut global_translations = TRANSLATIONS.write().unwrap();
    *global_translations = translations;
}

// Add about window translations
fn init_about_translations(en: &mut HashMap<String, String>, zh: &mut HashMap<String, String>) {
    // About window - English
    en.insert("app_name".to_string(), "Bird Player".to_string());
    en.insert(
        "app_description".to_string(),
        "A music player dedicated for local MP3 files, inspired by the things from the amazing 2000s golden age.".to_string(),
    );
    en.insert("features".to_string(), "Features:".to_string());
    en.insert(
        "feature_1".to_string(),
        "• Playing music, simple and straightforward, no streaming bullshit.".to_string(),
    );
    en.insert(
        "feature_2".to_string(),
        "• A cassette mimic, with a focus on simplicity and clean design.".to_string(),
    );
    en.insert(
        "feature_3".to_string(),
        "• Local Music library with ID3 editable tag support".to_string(),
    );
    en.insert("feature_4".to_string(), "• Playlist management".to_string());
    en.insert(
        "contact_email".to_string(),
        "Contact: digimonkey@protonmail.com".to_string(),
    );

    // About window - Chinese
    zh.insert("app_name".to_string(), "小鸟播放器".to_string());
    zh.insert(
        "app_description".to_string(),
        "一款专为本地 MP3 文件设计的音乐播放器，灵感来源于2000年代的黄金时代。".to_string(),
    );
    zh.insert("features".to_string(), "特点：".to_string());
    zh.insert(
        "feature_1".to_string(),
        "• 播放音乐，简单直接，拒绝狗屁流媒体。".to_string(),
    );
    zh.insert(
        "feature_2".to_string(),
        "• 模拟磁带播放器，注重简约和清晰的设计。".to_string(),
    );
    zh.insert(
        "feature_3".to_string(),
        "• 本地音乐库，支持ID3标签编辑".to_string(),
    );
    zh.insert("feature_4".to_string(), "• 播放列表管理".to_string());
    zh.insert(
        "contact_email".to_string(),
        "联系邮箱: digimonkey@protonmail.com".to_string(),
    );
}

// Set the current language
pub fn set_language(lang: Language) {
    let mut current = CURRENT_LANGUAGE.write().unwrap();
    *current = lang;
}

// Get the current language
pub fn get_language() -> Language {
    *CURRENT_LANGUAGE.read().unwrap()
}

// Translate a key to the current language
pub fn t(key: &str) -> String {
    let lang = *CURRENT_LANGUAGE.read().unwrap();
    let translations = TRANSLATIONS.read().unwrap();

    if let Some(lang_translations) = translations.get(&lang) {
        if let Some(translation) = lang_translations.get(key) {
            return translation.clone();
        }
    }

    // Return the key if no translation is found
    key.to_string()
}

// Translate a key with format arguments
pub fn tf(key: &str, args: &[&str]) -> String {
    let translated_format = t(key);
    // Simple replacement of {} with arguments
    let mut result = translated_format;
    for arg in args {
        if let Some(pos) = result.find("{}") {
            result.replace_range(pos..pos + 2, arg);
        }
    }
    result
}
