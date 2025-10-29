use crate::app::t;

pub(crate) struct PlaylistLocalization {
    column_number: String,
    column_title: String,
    column_artist: String,
    column_album: String,
    column_lyrics: String,
    column_genre: String,
    menu_edit_title: String,
    menu_edit_artist: String,
    menu_edit_album: String,
    menu_edit_genre: String,
    menu_remove_from_playlist: String,
    remove_lyrics: String,
}

impl PlaylistLocalization {
    pub fn current() -> Self {
        Self {
            column_number: t("column_number"),
            column_title: t("column_title"),
            column_artist: t("column_artist"),
            column_album: t("column_album"),
            column_lyrics: t("column_lyrics"),
            column_genre: t("column_genre"),
            menu_edit_title: t("edit_title"),
            menu_edit_artist: t("edit_artist"),
            menu_edit_album: t("edit_album"),
            menu_edit_genre: t("edit_genre"),
            menu_remove_from_playlist: t("remove_from_playlist"),
            remove_lyrics: t("remove_lyrics"),
        }
    }

    pub fn column_number(&self) -> &str {
        &self.column_number
    }

    pub fn column_title(&self) -> &str {
        &self.column_title
    }

    pub fn column_artist(&self) -> &str {
        &self.column_artist
    }

    pub fn column_album(&self) -> &str {
        &self.column_album
    }

    pub fn column_lyrics(&self) -> &str {
        &self.column_lyrics
    }

    pub fn column_genre(&self) -> &str {
        &self.column_genre
    }

    pub fn menu_edit_title(&self) -> &str {
        &self.menu_edit_title
    }

    pub fn menu_edit_artist(&self) -> &str {
        &self.menu_edit_artist
    }

    pub fn menu_edit_album(&self) -> &str {
        &self.menu_edit_album
    }

    pub fn menu_edit_genre(&self) -> &str {
        &self.menu_edit_genre
    }

    pub fn menu_remove_from_playlist(&self) -> &str {
        &self.menu_remove_from_playlist
    }

    pub fn remove_lyrics(&self) -> &str {
        &self.remove_lyrics
    }
}
