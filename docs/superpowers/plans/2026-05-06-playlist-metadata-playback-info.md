# Playlist Metadata and Playback Info Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add metadata fields (description, timestamps) to playlists and display contextual playback information in the player UI's top-right corner.

**Architecture:** Extend Playlist data model with metadata fields, migrate database schema, extract audio technical info at playback time, and create a new UI component to display contextual information based on playback state.

**Tech Stack:** Rust, egui, rusqlite, symphonia, lofty

---

## File Structure

**New files:**
- `src/app/components/playback_info_panel.rs` - UI component for displaying playback context

**Modified files:**
- `src/lib/playlist.rs` - Add metadata fields and accessor methods
- `src/app/db.rs` - Database schema migration
- `src/lib/player.rs` - Add audio technical info fields
- `src/lib/audio/loader.rs` - Extract technical info from codec
- `src/lib/messaging.rs` - Add AudioEvent variant for technical info
- `src/app/core.rs` - Handle technical info event
- `src/app/components/mod.rs` - Export new component
- `src/app/components/player_component.rs` - Integrate playback info panel

---

### Task 1: Add Playlist Metadata Fields

**Files:**
- Modify: `src/lib/playlist.rs:9-18`

- [ ] **Step 1: Add metadata fields to Playlist struct**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: Option<i64>,
    name: Option<String>,
    pub tracks: Vec<LibraryItem>,
    pub selected: Option<LibraryItem>,
    #[serde(skip_serializing, skip_deserializing)]
    pub selected_indices: HashSet<usize>,
    #[serde(skip)]
    pub is_dirty: bool,
    description: Option<String>,
    created_at: i64,
    updated_at: i64,
}
```

- [ ] **Step 2: Update Playlist::new() to initialize timestamps**

In `src/lib/playlist.rs`, find `Playlist::new()` around line 27 and modify:

```rust
pub fn new() -> Self {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    
    Self {
        id: None,
        name: None,
        tracks: vec![],
        selected: None,
        selected_indices: HashSet::new(),
        is_dirty: true,
        description: None,
        created_at: now,
        updated_at: now,
    }
}
```

- [ ] **Step 3: Add accessor methods for metadata**

Add after `get_name()` method around line 46:

```rust
pub fn description(&self) -> Option<&str> {
    self.description.as_deref()
}

pub fn set_description(&mut self, description: Option<String>) {
    self.description = description;
    self.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    self.is_dirty = true;
}

pub fn created_at(&self) -> i64 {
    self.created_at
}

pub fn updated_at(&self) -> i64 {
    self.updated_at
}
```

- [ ] **Step 4: Update set_name to update timestamp**

Modify `set_name()` around line 38:

```rust
pub fn set_name(&mut self, name: String) {
    self.name = Some(name);
    self.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    self.is_dirty = true;
}
```

- [ ] **Step 5: Update add() to update timestamp**

Modify `add()` around line 48:

```rust
pub fn add(&mut self, track: LibraryItem) {
    self.tracks.push(track);
    self.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    self.is_dirty = true;
}
```

- [ ] **Step 6: Update remove() to update timestamp**

Modify `remove()` around line 54:

```rust
pub fn remove(&mut self, idx: usize) {
    self.tracks.remove(idx);
    self.selected_indices.remove(&idx);
    self.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    self.is_dirty = true;

    // Update indices greater than the removed index
    let mut to_remove = Vec::new();
    let mut to_add = Vec::new();

    for &i in &self.selected_indices {
        if i > idx {
            to_remove.push(i);
            to_add.push(i - 1);
        }
    }

    for i in to_remove {
        self.selected_indices.remove(&i);
    }

    for i in to_add {
        self.selected_indices.insert(i);
    }
}
```

- [ ] **Step 7: Update reorder() to update timestamp**

Modify `reorder()` around line 80:

```rust
pub fn reorder(&mut self, current_pos: usize, destination_pos: usize) {
    let track = self.tracks.remove(current_pos);
    self.tracks.insert(destination_pos, track);
    self.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    self.is_dirty = true;

    // Update selected indices after reordering
    let mut new_selected = HashSet::new();

    for &idx in &self.selected_indices {
        if idx == current_pos {
            new_selected.insert(destination_pos);
        } else if (idx < current_pos && idx < destination_pos)
            || (idx > current_pos && idx > destination_pos)
        {
            new_selected.insert(idx);
        } else if idx < current_pos && idx >= destination_pos {
            new_selected.insert(idx + 1);
        } else if idx > current_pos && idx <= destination_pos {
            new_selected.insert(idx - 1);
        }
    }

    self.selected_indices = new_selected;
}
```

- [ ] **Step 8: Commit**

```bash
git add src/lib/playlist.rs
git commit -m "feat(playlist): add metadata fields (description, timestamps)"
```

---

### Task 2: Database Schema Migration

**Files:**
- Modify: `src/app/db.rs:125-132`
- Modify: `src/app/db.rs:50-70` (schema version check)

- [ ] **Step 1: Add migration logic to initialize_database()**

Find the `initialize_database()` function around line 50 in `src/app/db.rs`. After creating the `playlists` table (around line 132), add migration logic:

```rust
// Check schema version and run migrations
let current_version: i32 = connection
    .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
    .unwrap_or(1);

if current_version < 2 {
    tracing::info!("Running database migration to version 2");
    
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    
    // Add new columns to playlists table
    connection.execute(
        "ALTER TABLE playlists ADD COLUMN description TEXT",
        [],
    )?;
    
    connection.execute(
        "ALTER TABLE playlists ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0",
        [],
    )?;
    
    connection.execute(
        "ALTER TABLE playlists ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        [],
    )?;
    
    // Set timestamps for existing playlists
    connection.execute(
        "UPDATE playlists SET created_at = ?1, updated_at = ?1 WHERE created_at = 0",
        [current_time],
    )?;
    
    // Update schema version
    connection.execute("UPDATE schema_version SET version = 2", [])?;
    
    tracing::info!("Database migration to version 2 completed");
}
```

- [ ] **Step 2: Update Playlist::save_to_db() to include new fields**

Find `save_to_db()` method around line 150 in `src/lib/playlist.rs`. Update the INSERT and UPDATE statements:

```rust
pub fn save_to_db(&mut self, conn: &Arc<Mutex<Connection>>) -> SqlResult<()> {
    if !self.is_dirty {
        return Ok(());
    }

    let mut conn = conn.lock().unwrap();

    // Start a transaction
    let tx = conn.transaction()?;

    // Insert or update the playlist record
    let playlist_id = match self.id {
        Some(id) => {
            // Update existing playlist
            tx.execute(
                "UPDATE playlists SET name = ?1, description = ?2, updated_at = ?3 WHERE id = ?4",
                rusqlite::params![self.name, self.description, self.updated_at, id],
            )?;
            id
        }
        None => {
            // Insert new playlist
            tx.execute(
                "INSERT INTO playlists (name, description, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![self.name, self.description, self.created_at, self.updated_at],
            )?;
            tx.last_insert_rowid()
        }
    };

    // Clear existing playlist items
    tx.execute(
        "DELETE FROM playlist_items WHERE playlist_id = ?1",
        rusqlite::params![playlist_id],
    )?;

    // Insert the tracks with their positions
    for (position, track) in self.tracks.iter().enumerate() {
        tx.execute(
            "INSERT INTO playlist_items (playlist_id, library_item_id, position) 
             VALUES (?1, ?2, ?3)",
            rusqlite::params![playlist_id, track.key().to_string(), position as i32],
        )?;
    }

    // Commit the transaction
    tx.commit()?;
    self.is_dirty = false;

    Ok(())
}
```

- [ ] **Step 3: Update save_to_db_and_update_id() similarly**

Find `save_to_db_and_update_id()` around line 202 and apply the same changes:

```rust
pub fn save_to_db_and_update_id(&mut self, conn: &Arc<Mutex<Connection>>) -> SqlResult<()> {
    if !self.is_dirty {
        return Ok(());
    }

    let mut conn = conn.lock().unwrap();
    let tx = conn.transaction()?;

    let playlist_id = match self.id {
        Some(id) => {
            tx.execute(
                "UPDATE playlists SET name = ?1, description = ?2, updated_at = ?3 WHERE id = ?4",
                rusqlite::params![self.name, self.description, self.updated_at, id],
            )?;
            id
        }
        None => {
            tx.execute(
                "INSERT INTO playlists (name, description, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![self.name, self.description, self.created_at, self.updated_at],
            )?;
            tx.last_insert_rowid()
        }
    };

    if self.id.is_none() {
        self.id = Some(playlist_id);
    }

    tx.execute(
        "DELETE FROM playlist_items WHERE playlist_id = ?1",
        rusqlite::params![playlist_id],
    )?;

    for (position, track) in self.tracks.iter().enumerate() {
        tx.execute(
            "INSERT INTO playlist_items (playlist_id, library_item_id, position) 
             VALUES (?1, ?2, ?3)",
            rusqlite::params![playlist_id, track.key().to_string(), position as i32],
        )?;
    }

    tx.commit()?;
    self.is_dirty = false;

    Ok(())
}
```

- [ ] **Step 4: Update load_from_db() to load new fields**

Find `load_from_db()` around line 259 and update the SELECT statement:

```rust
pub fn load_from_db(conn: &Arc<Mutex<Connection>>, playlist_id: i64) -> SqlResult<Self> {
    let conn_guard = conn.lock().unwrap();

    // Get the playlist info
    let mut stmt = conn_guard.prepare(
        "SELECT id, name, description, created_at, updated_at FROM playlists WHERE id = ?1"
    )?;

    let mut playlist_rows = stmt.query(rusqlite::params![playlist_id])?;

    if let Some(row) = playlist_rows.next()? {
        let id: i64 = row.get(0)?;
        let name: Option<String> = row.get(1)?;
        let description: Option<String> = row.get(2)?;
        let created_at: i64 = row.get(3)?;
        let updated_at: i64 = row.get(4)?;

        // Create the playlist
        let mut playlist = Playlist {
            id: Some(id),
            name,
            tracks: vec![],
            selected: None,
            selected_indices: HashSet::new(),
            is_dirty: false,
            description,
            created_at,
            updated_at,
        };

        // Get the tracks (rest of the method remains the same)
        // ... existing track loading code ...
```

- [ ] **Step 5: Test database migration**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 6: Commit**

```bash
git add src/app/db.rs src/lib/playlist.rs
git commit -m "feat(db): add playlist metadata schema migration to version 2"
```

---

### Task 3: Add Audio Technical Info to Player

**Files:**
- Modify: `src/lib/player.rs:18-26`
- Modify: `src/lib/messaging.rs` (add AudioEvent variant)
- Modify: `src/lib/audio/loader.rs:88-99`
- Modify: `src/app/core.rs` (handle new event)

- [ ] **Step 1: Add technical info fields to Player struct**

In `src/lib/player.rs`, add fields after `duration` around line 25:

```rust
pub struct Player {
    pub selected_track: Option<LibraryItem>,
    pub track_state: TrackState,
    pub volume: f32,
    pub playback_mode: PlaybackMode,
    pub seek_to_timestamp: u64,
    pub duration: u64,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub codec: Option<String>,
}
```

- [ ] **Step 2: Initialize new fields in Player::new()**

Around line 43:

```rust
impl Player {
    pub fn new() -> Self {
        Self {
            selected_track: None,
            track_state: TrackState::Stopped,
            volume: 1.0,
            playback_mode: PlaybackMode::Normal,
            seek_to_timestamp: 0,
            duration: 0,
            sample_rate: None,
            channels: None,
            codec: None,
        }
    }
```

- [ ] **Step 3: Add AudioEvent::TechnicalInfo variant**

In `src/lib/messaging.rs`, find the `AudioEvent` enum and add:

```rust
pub enum AudioEvent {
    TotalTrackDuration(u64),
    CurrentTimestamp(u64),
    TrackEnded,
    TechnicalInfo {
        sample_rate: Option<u32>,
        channels: Option<u8>,
        codec: Option<String>,
    },
}
```

- [ ] **Step 4: Extract technical info in audio loader**

In `src/lib/audio/loader.rs`, after line 99 where `codec_params` is accessed, add:

```rust
// Extract technical info for UI display
let sample_rate = track.codec_params.sample_rate;
let channels = track.codec_params.channels.map(|ch| ch.count() as u8);
let codec = track.codec_params.codec.map(|c| format!("{:?}", c));

// Send technical info to UI
ui_tx
    .send(AudioEvent::TechnicalInfo {
        sample_rate,
        channels,
        codec,
    })
    .ok();
```

- [ ] **Step 5: Handle TechnicalInfo event in App**

In `src/app/core.rs`, find `pump_audio_events()` method and add handler:

```rust
AudioEvent::TechnicalInfo {
    sample_rate,
    channels,
    codec,
} => {
    let player = self.player_mut_ref();
    player.sample_rate = sample_rate;
    player.channels = channels;
    player.codec = codec;
}
```

- [ ] **Step 6: Test compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 7: Commit**

```bash
git add src/lib/player.rs src/lib/messaging.rs src/lib/audio/loader.rs src/app/core.rs
git commit -m "feat(player): add audio technical info extraction (sample rate, channels, codec)"
```

---

### Task 4: Create PlaybackInfoPanel Component

**Files:**
- Create: `src/app/components/playback_info_panel.rs`
- Modify: `src/app/components/mod.rs`

- [ ] **Step 1: Create playback_info_panel.rs**

Create new file `src/app/components/playback_info_panel.rs`:

```rust
use super::AppComponent;
use crate::app::style::tokens;
use crate::app::App;
use eframe::egui::{self, Align, Layout, RichText};

pub struct PlaybackInfoPanel;

impl AppComponent for PlaybackInfoPanel {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        // Check if we have a playing playlist
        let playing_playlist_idx = ctx.app_settings.playing_playlist_idx;
        
        if let Some(playlist_idx) = playing_playlist_idx {
            // Case 1: Playlist is playing
            Self::render_playlist_info(ctx, ui, playlist_idx);
        } else {
            // Case 2: Single track (no playlist)
            Self::render_track_info(ctx, ui);
        }
    }
}

impl PlaybackInfoPanel {
    fn render_playlist_info(ctx: &App, ui: &mut egui::Ui, playlist_idx: usize) {
        if let Some(playlist) = ctx.playlists.get(playlist_idx) {
            ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                let weak_color = ui.visuals().weak_text_color();
                
                // Playlist name
                if let Some(name) = playlist.get_name() {
                    ui.label(
                        RichText::new(&name)
                            .size(tokens::text::SM)
                            .color(weak_color),
                    );
                }
                
                // Track position
                if let Some(selected_track) = &ctx.player_ref().selected_track {
                    if let Some(pos) = playlist.get_pos(selected_track) {
                        let total = playlist.tracks.len();
                        ui.label(
                            RichText::new(format!("Track {:02}/{:02}", pos + 1, total))
                                .size(tokens::text::SM)
                                .color(weak_color),
                        );
                    }
                }
                
                // Description preview (first 30 chars)
                if let Some(desc) = playlist.description() {
                    let preview = if desc.len() > 30 {
                        format!("{}...", &desc[..30])
                    } else {
                        desc.to_string()
                    };
                    ui.label(
                        RichText::new(preview)
                            .size(tokens::text::SM)
                            .color(weak_color),
                    );
                }
            });
        }
    }
    
    fn render_track_info(ctx: &App, ui: &mut egui::Ui) {
        let player = ctx.player_ref();
        
        if let Some(track) = &player.selected_track {
            ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                let weak_color = ui.visuals().weak_text_color();
                
                // Album · Year
                let mut album_line = String::new();
                if let Some(album) = track.album() {
                    album_line.push_str(&album);
                }
                if let Some(year) = track.year() {
                    if !album_line.is_empty() {
                        album_line.push_str(" · ");
                    }
                    album_line.push_str(&year.to_string());
                }
                if !album_line.is_empty() {
                    ui.label(
                        RichText::new(album_line)
                            .size(tokens::text::SM)
                            .color(weak_color),
                    );
                }
                
                // Genre
                if let Some(genre) = track.genre() {
                    ui.label(
                        RichText::new(genre)
                            .size(tokens::text::SM)
                            .color(weak_color),
                    );
                }
                
                // Format · Sample Rate · Channels
                let mut tech_line = String::new();
                if let Some(codec) = &player.codec {
                    tech_line.push_str(codec);
                }
                if let Some(sample_rate) = player.sample_rate {
                    if !tech_line.is_empty() {
                        tech_line.push_str(" · ");
                    }
                    tech_line.push_str(&format!("{:.1}kHz", sample_rate as f32 / 1000.0));
                }
                if let Some(channels) = player.channels {
                    if !tech_line.is_empty() {
                        tech_line.push_str(" · ");
                    }
                    let channel_str = match channels {
                        1 => "Mono",
                        2 => "Stereo",
                        n => return format!("{}ch", n),
                    };
                    tech_line.push_str(channel_str);
                }
                if !tech_line.is_empty() {
                    ui.label(
                        RichText::new(tech_line)
                            .size(tokens::text::SM)
                            .color(weak_color),
                    );
                }
            });
        }
    }
}
```

- [ ] **Step 2: Export component in mod.rs**

In `src/app/components/mod.rs`, add:

```rust
mod playback_info_panel;
pub use playback_info_panel::PlaybackInfoPanel;
```

- [ ] **Step 3: Test compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/app/components/playback_info_panel.rs src/app/components/mod.rs
git commit -m "feat(ui): add PlaybackInfoPanel component for contextual playback info"
```

---

### Task 5: Integrate PlaybackInfoPanel into Player UI

**Files:**
- Modify: `src/app/components/player_component.rs:96-167`

- [ ] **Step 1: Import PlaybackInfoPanel**

At the top of `src/app/components/player_component.rs`, add to imports:

```rust
use super::playback_info_panel::PlaybackInfoPanel;
```

- [ ] **Step 2: Add PlaybackInfoPanel to player layout**

Find the horizontal layout around line 97 that contains `CassetteComponent` and track info. Modify to add the info panel on the right:

```rust
// ── Top row: cover + track info + playback info ────────────────
ui.horizontal(|ui| {
    CassetteComponent::add(ctx, ui);

    ui.allocate_ui_with_layout(
        vec2(ui.available_width() * 0.6, ui.available_height()),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            if let Some(track) = &selected_track {
                let title = track.title.as_deref().unwrap_or("unknown title");
                let artist = track.artist.as_deref().unwrap_or("unknown artist");

                let format_time = |timestamp: u64| -> String {
                    let total_seconds = timestamp / 1000;
                    let minutes = total_seconds / 60;
                    let seconds = total_seconds % 60;
                    format!("{:02}:{:02}", minutes, seconds)
                };

                ui.add(
                    egui::Label::new(
                        RichText::new(title).size(tokens::text::LG).strong(),
                    )
                    .truncate(),
                );
                ui.add(egui::Label::new(RichText::new(artist).weak()).truncate());
                ui.add(
                    egui::Label::new(
                        RichText::new(format!(
                            "{} / {}",
                            format_time(seek_to_timestamp),
                            format_time(duration),
                        ))
                        .size(tokens::text::SM)
                        .weak(),
                    )
                    .truncate(),
                );
                if !current_playlist_name.is_empty() {
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!(
                                "{}: {}",
                                t("playlist_label"),
                                current_playlist_name,
                            ))
                            .size(tokens::text::SM)
                            .weak(),
                        )
                        .truncate(),
                    );
                }
            } else {
                ui.add(egui::Label::new(
                    RichText::new(t("no_track")).size(tokens::text::LG).strong(),
                ));
                let hint = if has_tracks_in_playlist {
                    t("select_track")
                } else if current_playlist_idx.is_some() {
                    t("add_tracks")
                } else {
                    t("create_playlist")
                };
                ui.add(egui::Label::new(RichText::new(hint).weak()));
            }
        },
    );
    
    // Add playback info panel on the right
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), ui.available_height()),
        egui::Layout::top_down(egui::Align::RIGHT),
        |ui| {
            PlaybackInfoPanel::add(ctx, ui);
        },
    );
});
```

- [ ] **Step 3: Test compilation**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 4: Test the UI**

Run: `cargo run`
Expected: 
- Right corner shows playlist info when playing from a playlist
- Right corner shows album/technical info when playing a single track
- Info updates when switching tracks

- [ ] **Step 5: Commit**

```bash
git add src/app/components/player_component.rs
git commit -m "feat(ui): integrate PlaybackInfoPanel into player component"
```

---

## Self-Review Checklist

**Spec coverage:**
- ✅ Playlist metadata fields (description, timestamps) - Task 1
- ✅ Database schema migration - Task 2
- ✅ Audio technical info extraction - Task 3
- ✅ PlaybackInfoPanel component - Task 4
- ✅ UI integration - Task 5

**Placeholder scan:**
- ✅ No TBD, TODO, or "implement later"
- ✅ All code blocks are complete
- ✅ All commands have expected output

**Type consistency:**
- ✅ `description: Option<String>` used consistently
- ✅ `created_at: i64` and `updated_at: i64` used consistently
- ✅ `sample_rate: Option<u32>`, `channels: Option<u8>`, `codec: Option<String>` used consistently

**Missing from spec:**
- None - all requirements covered

---

## Execution Complete

All tasks defined. Plan is ready for execution.

