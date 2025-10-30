# Audio Backend Requirements

Bird Player supports multiple audio backends depending on your operating system:

## Linux

### PulseAudio (Recommended)
PulseAudio is the recommended audio backend for Linux and provides the best experience.

#### Dependencies:
- `libpulse-dev` - PulseAudio development libraries
- `libasound2-dev` - ALSA development libraries (often needed by PulseAudio)

#### How to install:
```bash
# Ubuntu/Debian
sudo apt-get install libpulse-dev libasound2-dev

# Fedora
sudo dnf install pulseaudio-libs-devel alsa-lib-devel

# Arch Linux
sudo pacman -S libpulse alsa-lib
```

#### Building with PulseAudio support:
```bash
cargo build --features pulseaudio
```

### CPAL (Fallback)
If PulseAudio is not available, Bird Player will use CPAL (Cross-Platform Audio Library) which can use ALSA directly.

#### Dependencies:
- `libasound2-dev` - ALSA development libraries

#### How to install:
```bash
# Ubuntu/Debian
sudo apt-get install libasound2-dev

# Fedora
sudo dnf install alsa-lib-devel

# Arch Linux
sudo pacman -S alsa-lib
```

#### Building without PulseAudio:
```bash
cargo build
```

### Troubleshooting

#### No sound in PulseAudio mode
- Check if PulseAudio is running: `pulseaudio --check`
- Try restarting PulseAudio: `pulseaudio -k && pulseaudio --start`
- Verify your system volume is not muted

#### No sound in CPAL/ALSA mode
- Check if your user has permission to access audio devices
- Run `alsamixer` to check volume levels
- Ensure ALSA is properly configured with `aplay -l`

## macOS and Windows

On macOS and Windows, Bird Player automatically uses the system's native audio APIs through CPAL, so no additional configuration is needed. 
