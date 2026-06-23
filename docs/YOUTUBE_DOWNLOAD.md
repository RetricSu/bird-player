# YouTube Download Notes

Bird Player can download audio from YouTube URLs using [yt-dlp](https://github.com/yt-dlp/yt-dlp).

## How yt-dlp is located

The app tries to find a working `yt-dlp` binary in this order:

1. A managed copy downloaded by the app itself:
   - macOS: `~/Library/Application Support/bird-player/bin/yt-dlp`
   - Windows: `%LOCALAPPDATA%\bird-player\bin\yt-dlp.exe`
   - Linux: `$XDG_DATA_HOME/bird-player/bin/yt-dlp` or `~/.local/share/bird-player/bin/yt-dlp`
2. Common installation paths:
   - macOS: `~/.local/bin/yt-dlp`, `/opt/homebrew/bin/yt-dlp`, `/usr/local/bin/yt-dlp`
   - Linux: `~/.local/bin/yt-dlp`, `/usr/local/bin/yt-dlp`, `/usr/bin/yt-dlp`
   - Windows: `%USERPROFILE%\.local\bin\yt-dlp.exe`, `C:\Program Files\yt-dlp\yt-dlp.exe`
3. Any `yt-dlp` available via the process `PATH`.
4. If none of the above work, the latest official release binary is downloaded from GitHub.

## PATH forwarding

When Bird Player launches `yt-dlp`, it forwards the user's shell `PATH` to the `yt-dlp` subprocess. This lets `yt-dlp` find companion tools such as `ffmpeg`, `ffprobe`, and JavaScript runtimes (Deno, Node, Bun) that the user already has installed, even when the GUI app was started from Finder / Launchpad / Explorer and inherited a minimal system `PATH`.

### Limitations of PATH forwarding

- **Requires a login shell.** The app runs `$SHELL -l -c 'echo $PATH'` to obtain the user's PATH. If the user's shell configuration is broken, extremely slow, or does not export `PATH` during a login shell, the fallback process PATH is used.
- **Depends on user-installed tools.** If the user has not installed `ffmpeg`, `ffprobe`, or a supported JS runtime, `yt-dlp` will still fail.
- **JS runtime for YouTube.** Recent yt-dlp versions need a JavaScript runtime (Deno is recommended, Node and QuickJS are also supported) to fully extract YouTube formats. Install one and make sure it is available in your shell PATH.
- **Cross-platform shell differences.** The implementation falls back to `/bin/sh` if `$SHELL` is not set. On Windows there is no Unix-style login shell, so PATH forwarding has no effect there.
- **Not a self-contained bundle.** This approach deliberately does not bundle `ffmpeg` or Deno into the app, keeping download sizes small. If you need a fully self-contained distribution, you would need to bundle those binaries separately.

## Required companion tools

For best results make sure these are installed and available in your shell PATH:

- [yt-dlp](https://github.com/yt-dlp/yt-dlp#installation)
- [ffmpeg](https://ffmpeg.org/download.html) (including `ffprobe`)
- [Deno](https://docs.deno.com/runtime/getting_started/installation/) (recommended JS runtime for YouTube extraction)

## Troubleshooting

- `ERROR: Postprocessing: ffprobe and ffmpeg not found` - install ffmpeg and ensure it is in your shell PATH.
- `WARNING: [youtube] No supported JavaScript runtime could be found` - install Deno or Node and ensure it is in your shell PATH.
- `yt-dlp was not found` - install yt-dlp or allow the app to download it automatically from GitHub.
