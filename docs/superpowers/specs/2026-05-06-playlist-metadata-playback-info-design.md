---
name: Playlist Metadata and Playback Info Panel
description: Add metadata fields to playlists and display contextual playback information in the player UI
type: feature
date: 2026-05-06
---

# Playlist Metadata and Playback Info Panel

## Overview

Add metadata fields (description, timestamps) to playlists and display contextual information in the player UI's top-right corner. When a playlist is playing, show playlist context; when playing a single track, show album/audio technical info.

## Goals

1. Allow users to add descriptions to playlists for organization and context
2. Track playlist creation and modification times
3. Display relevant playback context in the player UI without cluttering the interface
4. Provide audio technical information (sample rate, channels, codec) for users who care about quality

## Non-Goals

- Playlist cover images (deferred to future work)
- Total playlist duration (requires storing duration in LibraryItem, deferred)
- Album-level metadata beyond what's already in LibraryItem
- Playlist sorting/filtering by metadata (can be added later if needed)

## Data Model Changes

### Playlist Structure

Add three new fields to `Playlist` struct in `src/lib/playlist.rs`:

```rust
pub struct Playlist {
    pub id: Option<i64>,
    name: Option<String>,
    pub tracks: Vec<LibraryItem>,
    pub selected: Option<LibraryItem>,
    #[serde(skip_serializing, skip_deserializing)]
    pub selected_indices: HashSet<usize>,
    #[serde(skip)]
    pub is_dirty: bool,
    
    // New fields
    description: Option<String>,
    created_at: i64,  // Unix timestamp in milliseconds
    updated_at: i64,  // Unix timestamp in milliseconds
}
```

**Field semantics:**
- `description`: User-provided text describing the playlist's purpose, mood, or content
- `created_at`: Set once when `Playlist::new()` is called
- `updated_at`: Updated whenever:
  - `name` or `description` changes
  - Tracks are added, removed, or reordered
  - NOT updated for: selection changes, playback state changes

### Database Schema

Modify `playlists` table in `src/app/db.rs`:

```sql
ALTER TABLE playlists ADD COLUMN description TEXT;
ALTER TABLE playlists ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0;
ALTER TABLE playlists ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0;
```

**Migration strategy:**
- Check schema version in `initialize_database()`
- Increment schema version from 1 to 2
- For existing playlists, set `created_at` and `updated_at` to migration timestamp
- `description` defaults to NULL for existing playlists

### Player State

Add audio technical info fields to `Player` struct in `src/lib/player.rs`:

```rust
pub struct Player {
    // ... existing fields ...
    
    // New fields (populated when track loads)
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub codec: Option<String>,
}
```

These fields are populated in `src/lib/audio/loader.rs` when a track loads, extracted from `symphonia`'s `codec_params`.

## UI Components

### New Component: PlaybackInfoPanel

**Location:** `src/app/components/playback_info_panel.rs`

**Responsibility:** Render contextual playback information in the player UI's top-right corner.

**Display logic:**

**Case 1: Playlist is playing** (when `ctx.app_settings.playing_playlist_idx.is_some()`)
```
Playlist Name
Track 03/12
Description preview... (first 30 chars if present)
```

**Case 2: Single track (no playlist)** (when `ctx.app_settings.playing_playlist_idx.is_none()`)
```
Album · 2024
Rock
FLAC · 44.1kHz · Stereo
```

**Styling:**
- Font: `tokens::text::SM`
- Color: `ui.visuals().weak_text_color()`
- Alignment: Right-aligned
- Spacing: Compact vertical stack with `tokens::spacing::XS`

**Integration:**
Modify `src/app/components/player_component.rs` line ~97 (the horizontal layout with cassette + track info) to add `PlaybackInfoPanel` on the right side.

## Implementation Details

### Timestamp Management

Use `std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64` to generate timestamps.

Update `updated_at` in:
- `Playlist::set_name()`
- `Playlist::set_description()` (new method)
- `Playlist::add()`
- `Playlist::remove()`
- `Playlist::reorder()`

### Audio Technical Info Extraction

In `src/lib/audio/loader.rs`, after successfully loading a track (around line 88-99 where `codec_params` is accessed):

```rust
// Extract technical info for UI display
let sample_rate = track.codec_params.sample_rate;
let channels = track.codec_params.channels.map(|ch| ch.count() as u8);
let codec = track.codec_params.codec.map(|c| format!("{:?}", c));

// Send to player state via AudioEvent
ui_tx.send(AudioEvent::TechnicalInfo { sample_rate, channels, codec }).ok();
```

Add new `AudioEvent::TechnicalInfo` variant and handle it in `src/app/core.rs` to update `Player` state.

### Database Migration

In `src/app/db.rs`, modify `initialize_database()`:

1. Check current schema version
2. If version < 2, run migration:
   ```rust
   let current_time = SystemTime::now()
       .duration_since(UNIX_EPOCH)
       .unwrap()
       .as_millis() as i64;
   
   conn.execute("ALTER TABLE playlists ADD COLUMN description TEXT", [])?;
   conn.execute("ALTER TABLE playlists ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0", [])?;
   conn.execute("ALTER TABLE playlists ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0", [])?;
   
   // Set timestamps for existing playlists
   conn.execute(
       "UPDATE playlists SET created_at = ?1, updated_at = ?1 WHERE created_at = 0",
       [current_time]
   )?;
   
   // Update schema version
   conn.execute("UPDATE schema_version SET version = 2", [])?;
   ```

### Playlist Save/Load

Update `Playlist::save_to_db()` and `Playlist::load_from_db()` in `src/lib/playlist.rs` to include new fields in INSERT/UPDATE/SELECT statements.

## Testing Considerations

- Verify migration works on existing databases
- Test that `updated_at` updates correctly for all modification operations
- Test that `updated_at` does NOT update for selection/playback changes
- Verify UI displays correct info for both playlist and single-track cases
- Test with tracks that have missing metadata (no album, no genre, etc.)
- Test with audio files that don't expose sample rate/channels

## Future Enhancements

- Playlist cover images
- Total playlist duration (requires storing duration in LibraryItem)
- Playlist sorting by created_at/updated_at
- Playlist search by description
- Batch edit playlist metadata
