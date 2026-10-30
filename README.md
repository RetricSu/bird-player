# Bird Player

A MP3 music player built with [egui](https://github.com/emilk/egui) and Rust, featuring a nostalgic 2000s-inspired interface. Bird Player is originally forked from [music-player](https://github.com/notryanb/music-player) by [notryanb](https://github.com/notryanb) and try to make it a serious daily-used music player. If you miss the golden era of desktop music players, this might be for you!

<div style="display: flex; width: 100%;">
    <img src="./docs/conver.png" width="50%" alt="Cover">
    <img src="./docs/cover-black.jpg" width="50%" alt="Cover Black">
</div>

> [!NOTE]
> Development builds use an isolated `bird-player-dev` data directory. Production
> packages must enable the `production-data` feature explicitly.

## Features

- 🎨 Retro-inspired UI built with egui, reminiscent of classic 2000s music players
- 📁 Local music library and playlist management with familiar browsing experience
- 🏷️ ID3 tag management and editing support for music metadata
- 📱 Cross-platform support

## TODO

- [ ] Improve the resource usage and make it small and fast
- [ ] Add support for more audio formats and use `https://docs.rs/lofty/latest/lofty/` to manage metadata
- [ ] Add support for batch tag editing
- [x] Add support for lyrics searching and displaying
- [ ] Add support for speed control
- [ ] Implement a skin system

## Installation

### Release builds

Pushing a version tag (for example `git tag v0.2.0-rc4` followed by
`git push origin v0.2.0-rc4`) triggers the Release workflow. Update the package
version in `Cargo.toml` and `Cargo.lock` before tagging.

The workflow builds macOS DMGs for Apple Silicon and Intel, a Windows x64 ZIP,
and a Linux x64 tarball (Ubuntu 22.04 / glibc 2.35 or newer). Every package uses
`production-data`; Linux also enables PulseAudio. All builds must succeed before
the packages and `SHA256SUMS.txt` are uploaded to the GitHub Release. Tags with a
hyphen, such as `v0.2.0-rc4`, create a prerelease. Re-running the workflow replaces
the assets for that tag. A manual run on a branch builds downloadable workflow
artifacts without publishing a Release.

macOS apps use ad-hoc signing and are not notarized; Gatekeeper may require
approval in System Settings → Privacy & Security. Windows packages are unsigned.
Linux requires the GTK 3, ALSA, and PulseAudio runtime libraries (on Ubuntu 22.04:
`sudo apt install libgtk-3-0 libasound2 libpulse0`). YouTube download dependencies
listed below are installed separately.

### Prerequisites

- Rust 1.85 or higher
- Cargo package manager
- Audio system libraries (see [Audio Backends](/docs/AUDIO_BACKENDS.md) below)

### Building from Source

1. Clone the repository:
```bash
git clone https://github.com/yourusername/bird-player.git
cd bird-player
```

2. Build the project:
```bash
# With PulseAudio support (recommended for Linux)
cargo build --release --features pulseaudio

# Without PulseAudio (using CPAL directly)
cargo build --release
```

The compiled binary will be available in `target/release/bird-player`.

### macOS App Bundle

Build a macOS `.app` bundle with:

```bash
cargo bundle --release --features production-data
```

The bundle is written to `target/release/bundle/osx/Bird Player.app`. To install it locally, copy it to `/Applications`.

When the app is launched from Finder, Dock, or LaunchServices, it does not inherit the interactive shell `PATH`. Any bundled build that calls user-installed command line tools must set a runtime `PATH` explicitly. Bird Player currently does this for `yt-dlp` by adding common user binary directories such as `~/.local/bin`, `~/bin`, `/opt/homebrew/bin`, and `/usr/local/bin` before spawning the command. Keep this in mind when adding future external tools, otherwise a tool that works in Terminal may fail inside the packaged app with "not found".

For a local upgrade, use the safe installer:

```bash
./install_mac_app.sh
```

It builds with production data enabled, closes the running app, verifies and
backs up the SQLite database, preserves the old app bundle, installs the new
bundle, and verifies the database again after launch. Old app bundles use the
non-launchable `.app.backup` suffix so they cannot accidentally open the
production database.

Plain `cargo run` and `cargo run --release` intentionally use the isolated
development database. This prevents local UI testing from modifying the
installed app's music library and playlists.

### YouTube download dependencies and subtitle errors

Downloads require `yt-dlp`, `ffmpeg`/`ffprobe`, and a supported JavaScript runtime
(for example Deno). Keep yt-dlp current and install its browser impersonation
extra in the **same Python environment** as the executable used by Bird Player.
For a pipx installation (such as `~/.local/bin/yt-dlp` pointing into a pipx venv):

```bash
pipx install --force 'yt-dlp[default,curl-cffi]'
yt-dlp --version
yt-dlp --list-impersonate-targets
```

For a pip installation, use that environment's Python instead:

```bash
python -m pip install -U 'yt-dlp[default,curl-cffi]'
```

See the [yt-dlp dependency documentation](https://github.com/yt-dlp/yt-dlp#dependencies).
Hiding the old-version warning with `--no-update` does not update the downloader.

Subtitle requests are spaced five seconds apart. If only subtitle downloads
fail (including HTTP 429 rate limits), Bird Player retries once with subtitles
disabled, saves the audio, and logs the subtitle warning. Other download errors
still fail normally. YouTube can continue to rate-limit captions even after an
update; wait before trying captions again, or turn off **Download available
subtitles** to download audio immediately.

At startup, Bird Player creates and integrity-checks a SQLite snapshot in the
profile's `backups` directory, retaining the 10 newest snapshots. A build that
encounters an unknown or newer database schema stops with a visible warning;
it never drops or rebuilds existing user tables.

## Usage

1. Launch Bird Player:
```bash
cargo run --release
```

2. Use the file dialog to add your music directory
3. Browse and play your music collection
4. Enjoy your music with high-quality audio playback

## License

[MIT](LICENSE)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

The project is originally forked from [music-player](https://github.com/notryanb/music-player) by [notryanb](https://github.com/notryanb). Thanks to all the Rust crate authors whose work made this project possible.
