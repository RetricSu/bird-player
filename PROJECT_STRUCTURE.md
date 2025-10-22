**Project Overview**
- Rust 2021 workspace targeting a desktop `eframe` shell; entrypoint main.rs wires UI, audio, and persistence.
- UI, playback, and metadata management live under app, keeping the `App` state struct centralized.
- Audio decode/playback implemented with `symphonia` + `cpal`/PulseAudio, isolated in output.rs and resampler.rs.
- SQLite persistence via `rusqlite` resides in db.rs, with build-time version stamping handled in build.rs.

**Runtime Flow**
- `main()` boots tracing, opens the database, and constructs `App::load_basic()` before handing it to `eframe::run_native`.
- Background audio thread receives `AudioCommand` messages, decodes streams, and reports status back through `UiCommand`.
- Library import tasks spawn per-folder threads pushing `LibraryCommand` messages consumed during UI updates.
- `App::update()` (in app_impl.rs) orchestrates GUI layout, event handling, repaint scheduling, and async lyric fetch results.

**Core Modules**
- mod.rs defines the `App` struct, shared enums, settings persistence, heavy data loading, metadata editing, and lyric orchestration.
- player.rs holds playback state, queue navigation logic, volume/seek commands, and playback mode cycling.
- library.rs models folders, tracks, artwork, and database save/load routines for the local library.
- playlist.rs manages playlists, selection, ordering, and persistence, sharing `LibraryItem` references.
- lyrics.rs wraps lyric fetching, caching to ID3 tags, and parsing synced lyric timelines.

**UI Components**
- app_impl.rs implements `eframe::App`, composing panels, window chrome, player controls, library tree, playlists, and lyrics panel.
- mod.rs exposes reusable widgets like `player_component`, `library_component`, `playlist_table`, `lyrics_component`, etc.
- `window_chrome.rs` mimics custom title bar controls; `footer.rs` renders status/action hints.
- `playlist_tabs.rs` switches between playlists with rename/delete flows; `playlist_table.rs` renders track grids with inline editing.
- `cassette_component.rs` and `style/` assets provide themed visuals consistent with the 2000s-inspired aesthetic.

**Audio & Media**
- output.rs abstracts platform output: PulseAudio behind a feature flag, otherwise defaulting to `cpal`; handles volume scaling and buffering.
- `process_audio_cmd`, `load_file`, and decoder setup in main.rs coordinate playback lifecycle, seek, and timestamp reporting.
- resampler.rs offers FFT/linear resampling paths invoked when device/sample rates differ.
- icons supplies application icons referenced by `get_app_icon()` during native window creation.

**Persistence & Metadata**
- db.rs opens `bird-player.db` beneath the confy configuration folder, enforces schema versioning, and exposes a guarded `Connection`.
- `Library::save_to_db` / `load_from_db` sync folder paths, tracks, and artwork; playlists follow a similar pattern with ordering metadata.
- `App::save_state()` persists lightweight settings via `confy`, splitting heavy data to SQLite to avoid confy bloat.
- Metadata edits propagate to ID3 tags, SQLite rows, in-memory state, and playlists to keep UI and storage aligned.

**Lyrics & Localization**
- `LyricsService` runs a worker thread servicing `fetch_lyrics` requests via `mpsc`, hitting the lrclib API with artist/title queries.
- ID3 embedded lyrics are preferred; fetched results are cached back to the file for offline reuse.
- `Lyrics::parse_synced_lyrics` converts LRC-style timestamps to `LyricsLine` ranges, enabling time-synced highlighting.
- i18n.rs loads English/Chinese dictionaries, exposes helpers `t`/`tf`, and persists language choice through `AppSettings`.

**Other Assets**
- AUDIO_BACKENDS.md, README.md, and docs document backend support and generated crate docs.
- assets also houses theming resources used by the UI; doc contains rustdoc output from previous builds.
- rust-toolchain.toml pins the toolchain, ensuring consistent builds; build.rs injects `version_info` for about dialogs.
- Cargo.toml enables optional `pulseaudio` feature and applies a git patch for `confy` to support YAML-backed settings.

Let me know if you want a walkthrough of a particular flow (e.g., playlist persistence, synced lyric rendering, or adding new backends) or pointers on extending the UI.
