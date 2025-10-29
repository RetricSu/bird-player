use crate::app::App;

use super::{actions::PendingActions, services::PlaylistTableService};

pub(crate) fn apply(ctx: &mut App, playlist_idx: usize, actions: PendingActions) {
    let mut service = match PlaylistTableService::new(ctx, playlist_idx) {
        Some(service) => service,
        None => return,
    };

    let PendingActions {
        clear_lyrics,
        toggle_selection,
        metadata_updates,
        play_track,
        remove_track,
    } = actions;

    for key in clear_lyrics {
        service.clear_lyrics(key);
    }

    if let Some(idx) = toggle_selection {
        service.toggle_selection(idx);
    }

    for (idx, field, value) in metadata_updates {
        service.update_metadata(idx, &field, value);
    }

    if let Some(idx) = play_track {
        service.play_track(idx);
    }

    if let Some(idx) = remove_track {
        service.remove_track(idx);
    }
}
