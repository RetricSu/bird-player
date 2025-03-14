# Bird Player

A modern music player built with [egui](https://github.com/emilk/egui) and Rust, featuring a nostalgic 2000s-inspired interface. Bird Player combines the charm of classic music players with modern technology, providing a seamless experience for managing and playing your local music collection. If you miss the golden era of desktop music players, this is for you!

## Features

- 🎨 Retro-inspired UI built with egui, reminiscent of classic 2000s music players
- 🎵 Support for MP3 audio format
- 📁 Local music library management with familiar browsing experience
- 🏷️ ID3 tag management and editing support for music metadata
- ⚡ High-performance audio playback with CPAL
- 🎚️ Real-time audio processing and resampling
- 💾 Configuration persistence
- 📱 Cross-platform support
- 🌟 Nostalgic visual elements and animations
- 🎼 Classic playlist management system

## Installation

### Prerequisites

- Rust 1.70 or higher
- Cargo package manager

### Building from Source

1. Clone the repository:
```bash
git clone https://github.com/yourusername/bird-player.git
cd bird-player
```

2. Build the project:
```bash
cargo build --release
```

The compiled binary will be available in `target/release/bird-player`.

## Usage

1. Launch Bird Player:
```bash
cargo run --release
```

2. Use the file dialog to add your music directory
3. Browse and play your music collection
4. Enjoy your music with high-quality audio playback

## Configuration

Bird Player automatically saves your configuration and library state between sessions. The configuration file is stored in the standard system configuration directory using YAML format.

## Development

### Project Structure

- `src/main.rs`: Application entry point and main logic
- `src/output.rs`: Audio output handling
- `src/resampler.rs`: Audio resampling functionality
- `src/app/`: UI components and application state management

### Dependencies

- `eframe`: GUI framework
- `cpal`: Audio playback
- `symphonia`: Audio decoding
- `id3`: Music metadata handling
- `serde`: Configuration serialization
- Other utilities for file management and audio processing

## License

[Your chosen license]

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

Thanks to all the Rust crate authors whose work made this project possible.
