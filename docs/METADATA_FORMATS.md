# Audio Metadata and Multi-Format Support

This document details the metadata format limitations currently affecting Bird Player and outlines the necessary migration path for extending support beyond MP3s to high-fidelity formats like FLAC, OGG, and M4A.

## The Compatibility Issue with FLAC and ID3

Currently, the application relies heavily on the `id3` crate (e.g., `id3::Tag`) to parse and write audio metadata, extract album covers, and power the library import processes. This creates a hard blocker for FLAC support.

- **MP3 and ID3**: The MP3 format natively uses the ID3v2 standard for its metadata layout, which the `id3` crate handles perfectly.
- **FLAC Standard Compliance**: The official FLAC specification natively utilizes **Vorbis Comments** (part of the Ogg spec family) to store text metadata (Title, Artist, Album) and uses dedicated **FLAC PICTURE blocks** to store album art. 
- **The "Splicing" Hack**: While it is technically possible to forcefully attach an ID3 chunk to a FLAC file via non-standard tools, doing so violates the FLAC specification. Conforming music players and audio taggers will often fail to parse this "frankenstein" format, producing gibberish texts, causing crashes, or ignoring the tags entirely. Consequently, the `id3` crate simply cannot properly read or write compliant FLAC formats.

## Album Artwork Management

### Current Lifecycle
At present, Bird Player handles album covers via extraction rather than strictly acting as an embedded viewer/editor. 
1. When importing a track, the `LibraryImportService` iterates over any embedded pictures inside the MP3's ID3 chunk: `for pic in tag.pictures() { ... }`.
2. It extracts this image byte data and saves a local copy inside the app's `album_art` cache directory on disk.
3. The UI (egui) subsequently loads these cached image routes from disk to prevent real-time parsing bottlenecks.

### Desired UX & Limitations
A highly desired workflow is the ability to fetch new album art (e.g., via a scraping API like Douban) and **embed it directly back into the audio file**. 
Currently, the `MetadataEditor` service (`src/lib/services/metadata_editor.rs`) can write text tags back to the audio file:
```rust
tag.set_title(value); 
tag.write_to_path(path, Version::Id3v24);
```
However, the codebase currently lacks the logic to inject or update binary `Picture` frames back into the `id3::Tag`. Even if implemented, applying this to FLACs would fail for the reasons stated above.

## The Migration Path: Transitioning to `lofty`

To construct a robust, production-grade, multi-format audio engine, the underlying metadata dependency must be swapped out. The industry standard within the Rust ecosystem for this is the **`lofty`** crate.

### Why `lofty`?
1. **Unified Tag Abstraction (Unified API)**: `lofty` abstracts away the underlying metadata containers. `tag.set_title("Hello")` functions exactly the same programmatically, regardless of whether the file is an MP3 or a FLAC.
2. **Context-Aware Writing**: When instructed to save to disk, `lofty` intuitively knows the file's container natively:
   - For an MP3 file, it silently writes ID3 tags.
   - For a FLAC or OGG file, it translates the text into Vorbis Comments and creates the proper standard `PICTURE` blocks.
3. **Picture Abstraction**: Similar to text, it handles extracting and embedding artwork generically, mapping them correctly to ID3 picture types or FLAC picture block types.

### Implementation Checklist
- [ ] Replace `id3` with `lofty` in `Cargo.toml`.
- [ ] Update `LibraryImportService::parse_audio_file` to use `lofty::Probe::open(&path)?.read()`.
- [ ] Refactor `extract_album_art` to iterate through `tag.pictures()`, mapping lofty's generic picture format to our DB caching mechanism.
- [ ] Refactor `MetadataEditor::update_track_metadata` to retrieve the generic internal tagging format, modify it, and write it back contextually.
- [ ] Implement an "Update Cover" function utilizing `lofty` to convert an uploaded or downloaded image into standard embedded metadata chunks.
