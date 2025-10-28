use std::borrow::Cow;

use crate::app::library::LibraryItem;
use crate::app::t;

pub(crate) struct LocalizedFallbacks {
    unknown_title: String,
    unknown_artist: String,
    unknown_album: String,
    unknown_genre: String,
}

impl LocalizedFallbacks {
    pub fn current() -> Self {
        Self {
            unknown_title: t("unknown_title"),
            unknown_artist: t("unknown_artist"),
            unknown_album: t("unknown_album"),
            unknown_genre: t("unknown_genre"),
        }
    }

    pub fn title(&self) -> &str {
        &self.unknown_title
    }

    pub fn artist(&self) -> &str {
        &self.unknown_artist
    }

    pub fn album(&self) -> &str {
        &self.unknown_album
    }

    pub fn genre(&self) -> &str {
        &self.unknown_genre
    }
}

pub(crate) struct RowTexts<'a> {
    pub title: Cow<'a, str>,
    pub artist: Cow<'a, str>,
    pub album: Cow<'a, str>,
    pub genre: Cow<'a, str>,
}

pub(crate) fn extract_row_texts<'a>(
    track: &'a LibraryItem,
    fallbacks: &'a LocalizedFallbacks,
) -> RowTexts<'a> {
    RowTexts {
        title: track
            .title_ref()
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Borrowed(fallbacks.title())),
        artist: track
            .artist_ref()
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Borrowed(fallbacks.artist())),
        album: track
            .album_ref()
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Borrowed(fallbacks.album())),
        genre: track
            .genre_ref()
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Borrowed(fallbacks.genre())),
    }
}
