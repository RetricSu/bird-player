use crate::app::library::LibraryItem;
use crate::app::t;

pub(crate) struct RowTexts {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
}

pub(crate) fn extract_row_texts(track: &LibraryItem) -> RowTexts {
    RowTexts {
        title: track.title().unwrap_or_else(|| t("unknown_title")),
        artist: track.artist().unwrap_or_else(|| t("unknown_artist")),
        album: track.album().unwrap_or_else(|| t("unknown_album")),
        genre: track.genre().unwrap_or_else(|| t("unknown_genre")),
    }
}
