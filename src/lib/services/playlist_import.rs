use super::library_import::LibraryImportService;
use super::playlist_export::{PlaylistExportManifest, PlaylistExportTrack};
use crate::library::{LibraryItem, LibraryPathId};
use crate::playlist::Playlist;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use zip::ZipArchive;

const MAX_MANIFEST_BYTES: u64 = 2 * 1024 * 1024;
const MAX_COVER_BYTES: u64 = 12 * 1024 * 1024;
const MAX_TRACK_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MAX_TOTAL_TRACK_BYTES: u64 = 100 * 1024 * 1024 * 1024;
const MAX_TRACK_COUNT: usize = 10_000;

#[derive(Debug)]
pub struct PlaylistImportResult {
    pub playlist: Playlist,
    pub imported_items: Vec<LibraryItem>,
    pub import_dir: Option<PathBuf>,
}

pub struct PlaylistImportService;

impl PlaylistImportService {
    pub fn import_playlist(
        archive_path: &Path,
        destination_root: &Path,
        existing_items: &[LibraryItem],
        library_path_id: LibraryPathId,
        album_art_dir: &Path,
    ) -> Result<PlaylistImportResult, String> {
        let archive_file =
            File::open(archive_path).map_err(|err| format!("Failed to open playlist: {err}"))?;
        let mut archive = ZipArchive::new(archive_file)
            .map_err(|err| format!("Invalid playlist archive: {err}"))?;

        if archive.len() > MAX_TRACK_COUNT + 4 {
            return Err("Playlist archive contains too many files.".to_string());
        }

        let manifest_bytes =
            Self::read_entry_limited(&mut archive, "playlist.json", MAX_MANIFEST_BYTES)?;
        let mut manifest: PlaylistExportManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|err| format!("Invalid playlist metadata: {err}"))?;

        if manifest.format != "bird-player-playlist" {
            return Err("This is not a Bird Player playlist archive.".to_string());
        }
        if !matches!(manifest.format_version, 1 | 2) {
            return Err(format!(
                "Playlist format version {} is not supported.",
                manifest.format_version
            ));
        }
        if manifest.tracks.len() > MAX_TRACK_COUNT {
            return Err("Playlist contains too many tracks.".to_string());
        }

        manifest.tracks.sort_by_key(|track| track.position);

        let cover = if let Some(cover_path) = manifest.cover.as_deref() {
            Self::validate_archive_path(cover_path, "cover")?;
            let bytes = Self::read_entry_limited(&mut archive, cover_path, MAX_COVER_BYTES)?;
            image::load_from_memory(&bytes)
                .map_err(|err| format!("Playlist cover is not a valid image: {err}"))?;
            let mime_type = manifest
                .cover_mime_type
                .clone()
                .unwrap_or_else(|| Self::image_mime_from_path(cover_path).to_string());
            Some((mime_type, bytes))
        } else {
            None
        };

        fs::create_dir_all(destination_root)
            .map_err(|err| format!("Failed to create import destination: {err}"))?;
        let temp_dir = destination_root.join(format!(".bird-import-{}", uuid::Uuid::new_v4()));
        let final_dir = Self::unique_import_dir(
            destination_root,
            manifest.name.as_deref().unwrap_or("Imported Playlist"),
        );

        let import_result = Self::extract_and_build(
            &mut archive,
            &manifest,
            existing_items,
            library_path_id,
            album_art_dir,
            &temp_dir,
            &final_dir,
            cover,
        );

        if import_result.is_err() && temp_dir.exists() {
            let _ = fs::remove_dir_all(&temp_dir);
        }
        import_result
    }

    #[allow(clippy::too_many_arguments)]
    fn extract_and_build(
        archive: &mut ZipArchive<File>,
        manifest: &PlaylistExportManifest,
        existing_items: &[LibraryItem],
        library_path_id: LibraryPathId,
        album_art_dir: &Path,
        temp_dir: &Path,
        final_dir: &Path,
        cover: Option<(String, Vec<u8>)>,
    ) -> Result<PlaylistImportResult, String> {
        enum TrackSource {
            Existing(Box<LibraryItem>),
            Extracted(PathBuf),
        }

        let mut sources = Vec::with_capacity(manifest.tracks.len());
        let mut total_uncompressed = 0_u64;
        let mut created_temp_dir = false;

        for (idx, track) in manifest.tracks.iter().enumerate() {
            if !track.file_hash.is_empty() {
                if let Some(existing) = existing_items
                    .iter()
                    .find(|item| item.file_hash() == track.file_hash)
                {
                    sources.push(TrackSource::Existing(Box::new(existing.clone())));
                    continue;
                }
            }

            Self::validate_archive_path(&track.file, "tracks")?;
            if !Self::is_supported_audio_path(&track.file) {
                return Err(format!(
                    "Playlist contains an unsupported audio file: {}",
                    track.file
                ));
            }
            let entry_size = {
                let entry = archive
                    .by_name(&track.file)
                    .map_err(|_| format!("Track is missing from archive: {}", track.file))?;
                entry.size()
            };
            if entry_size > MAX_TRACK_BYTES {
                return Err(format!("Track is too large to import: {}", track.file));
            }
            total_uncompressed = total_uncompressed
                .checked_add(entry_size)
                .ok_or_else(|| "Playlist archive is too large.".to_string())?;
            if total_uncompressed > MAX_TOTAL_TRACK_BYTES {
                return Err("Playlist archive is too large to import safely.".to_string());
            }

            if !created_temp_dir {
                fs::create_dir_all(temp_dir)
                    .map_err(|err| format!("Failed to prepare playlist import: {err}"))?;
                created_temp_dir = true;
            }

            let source_name = Path::new(&track.file)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("track");
            let output_name = format!("{:03}-{}", idx + 1, Self::safe_file_name(source_name));
            let output_path = temp_dir.join(&output_name);
            let mut entry = archive
                .by_name(&track.file)
                .map_err(|_| format!("Track is missing from archive: {}", track.file))?;
            let mut output = File::create(&output_path)
                .map_err(|err| format!("Failed to create imported track: {err}"))?;
            std::io::copy(&mut entry, &mut output)
                .map_err(|err| format!("Failed to extract imported track: {err}"))?;
            output
                .flush()
                .map_err(|err| format!("Failed to finish imported track: {err}"))?;
            sources.push(TrackSource::Extracted(PathBuf::from(output_name)));
        }

        if created_temp_dir {
            fs::rename(temp_dir, final_dir)
                .map_err(|err| format!("Failed to finish playlist import: {err}"))?;
        }

        let mut playlist = Playlist::new();
        playlist.set_name(
            manifest
                .name
                .clone()
                .unwrap_or_else(|| "Imported Playlist".to_string()),
        );
        playlist.set_description(manifest.description.clone());
        playlist.set_curator(manifest.curator.clone());
        playlist.set_booklet(manifest.booklet.clone());
        if let Some((mime_type, data)) = cover {
            playlist.set_cover(Some(mime_type), Some(data));
        }

        let mut imported_items = Vec::new();
        for ((track, source), idx) in manifest
            .tracks
            .iter()
            .zip(sources.into_iter())
            .zip(0_usize..)
        {
            let item = match source {
                TrackSource::Existing(item) => *item,
                TrackSource::Extracted(relative_path) => {
                    let path = final_dir.join(relative_path);
                    let mut item = LibraryImportService::parse_audio_file_for_import(
                        &path,
                        library_path_id,
                        album_art_dir,
                    );
                    Self::apply_manifest_fallbacks(&mut item, track);
                    imported_items.push(item.clone());
                    item
                }
            };
            playlist.add(item);
            playlist.set_track_note(idx, track.curator_note.clone());
        }
        playlist.set_imported_timestamps(manifest.created_at, manifest.updated_at);

        Ok(PlaylistImportResult {
            playlist,
            imported_items,
            import_dir: created_temp_dir.then(|| final_dir.to_path_buf()),
        })
    }

    fn apply_manifest_fallbacks(item: &mut LibraryItem, track: &PlaylistExportTrack) {
        if item.title().is_none() {
            item.set_title(track.title.as_deref());
        }
        if item.artist().is_none() {
            item.set_artist(track.artist.as_deref());
        }
        if item.album().is_none() {
            item.set_album(track.album.as_deref());
        }
        if item.year().is_none() {
            item.set_year(track.year);
        }
        if item.genre().is_none() {
            item.set_genre(track.genre.as_deref());
        }
        if item.track_number().is_none() {
            item.set_track_number(track.track_number);
        }
        if item.lyrics().is_none() {
            item.replace_lyrics(track.lyrics.clone());
        }
        if !track.file_hash.is_empty() {
            item.set_file_hash(track.file_hash.clone());
        }
    }

    fn read_entry_limited(
        archive: &mut ZipArchive<File>,
        name: &str,
        max_bytes: u64,
    ) -> Result<Vec<u8>, String> {
        let entry = archive
            .by_name(name)
            .map_err(|_| format!("Playlist archive is missing {name}."))?;
        if entry.size() > max_bytes {
            return Err(format!("{name} is too large."));
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry
            .take(max_bytes + 1)
            .read_to_end(&mut bytes)
            .map_err(|err| format!("Failed to read {name}: {err}"))?;
        if bytes.len() as u64 > max_bytes {
            return Err(format!("{name} is too large."));
        }
        Ok(bytes)
    }

    fn validate_archive_path(path: &str, required_root: &str) -> Result<(), String> {
        let path = Path::new(path);
        let safe = !path.is_absolute()
            && path
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
            && path
                .components()
                .next()
                .is_some_and(|part| part.as_os_str() == required_root);
        if safe {
            Ok(())
        } else {
            Err("Playlist archive contains an unsafe file path.".to_string())
        }
    }

    fn unique_import_dir(destination_root: &Path, playlist_name: &str) -> PathBuf {
        let base = Self::safe_file_name(playlist_name);
        let mut candidate = destination_root.join(&base);
        let mut suffix = 2;
        while candidate.exists() {
            candidate = destination_root.join(format!("{base} {suffix}"));
            suffix += 1;
        }
        candidate
    }

    fn safe_file_name(name: &str) -> String {
        let safe = name
            .chars()
            .map(|ch| match ch {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
                ch if ch.is_control() => '-',
                ch => ch,
            })
            .collect::<String>();
        let safe = safe.trim().trim_matches('.').to_string();
        if safe.is_empty() {
            "Imported Playlist".to_string()
        } else {
            safe
        }
    }

    fn image_mime_from_path(path: &str) -> &'static str {
        match Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("png") => "image/png",
            Some("webp") => "image/webp",
            Some("gif") => "image/gif",
            _ => "image/jpeg",
        }
    }

    fn is_supported_audio_path(path: &str) -> bool {
        matches!(
            Path::new(path)
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("mp3" | "flac" | "wav" | "ogg" | "m4a")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::PlaylistImportService;
    use crate::library::{LibraryItem, LibraryPathId};
    use std::io::Write;

    #[test]
    fn archive_paths_must_stay_inside_expected_folder() {
        assert!(PlaylistImportService::validate_archive_path("tracks/one.mp3", "tracks").is_ok());
        assert!(PlaylistImportService::validate_archive_path("../one.mp3", "tracks").is_err());
        assert!(
            PlaylistImportService::validate_archive_path("cover/../../secret", "cover").is_err()
        );
        assert!(PlaylistImportService::validate_archive_path("/tracks/one.mp3", "tracks").is_err());
    }

    #[test]
    fn version_one_archive_reuses_an_existing_track() {
        let test_dir =
            std::env::temp_dir().join(format!("bird-player-import-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&test_dir).unwrap();
        let archive_path = test_dir.join("legacy.birdplaylist.zip");
        let archive_file = std::fs::File::create(&archive_path).unwrap();
        let mut archive = zip::ZipWriter::new(archive_file);
        archive
            .start_file("playlist.json", zip::write::FileOptions::default())
            .unwrap();
        archive
            .write_all(
                br#"{
                    "format": "bird-player-playlist",
                    "format_version": 1,
                    "name": "Legacy",
                    "description": "Still readable",
                    "avatar": null,
                    "created_at": 10,
                    "updated_at": 20,
                    "tracks": [{
                        "position": 1,
                        "file": "tracks/001-song.mp3",
                        "original_path": "/old/song.mp3",
                        "key": "old-key",
                        "file_hash": "stable-hash",
                        "title": "Song",
                        "artist": "Artist",
                        "album": null,
                        "year": null,
                        "genre": null,
                        "track_number": null,
                        "lyrics": null
                    }]
                }"#,
            )
            .unwrap();
        archive.finish().unwrap();

        let mut existing = LibraryItem::new(test_dir.join("song.mp3"), LibraryPathId::new(9));
        existing.set_file_hash("stable-hash".to_string());
        let result = PlaylistImportService::import_playlist(
            &archive_path,
            &test_dir.join("destination"),
            &[existing.clone()],
            LibraryPathId::new(10),
            &test_dir.join("art"),
        )
        .unwrap();

        assert_eq!(result.playlist.get_name().as_deref(), Some("Legacy"));
        assert_eq!(result.playlist.tracks[0].key(), existing.key());
        assert!(result.imported_items.is_empty());
        assert!(result.import_dir.is_none());

        std::fs::remove_dir_all(test_dir).unwrap();
    }
}
