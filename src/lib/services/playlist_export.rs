use crate::playlist::Playlist;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use zip::write::FileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

#[derive(Debug, Clone)]
pub struct PlaylistExportResult {
    pub output_path: PathBuf,
    pub track_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PlaylistExportManifest {
    pub(crate) format: String,
    pub(crate) format_version: u32,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) curator: Option<String>,
    #[serde(default)]
    pub(crate) booklet: Option<String>,
    #[serde(default)]
    pub(crate) cover: Option<String>,
    #[serde(default)]
    pub(crate) cover_mime_type: Option<String>,
    // Retained so v1 packages written by earlier Bird Player builds remain readable.
    #[serde(default)]
    pub(crate) avatar: Option<String>,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
    pub(crate) tracks: Vec<PlaylistExportTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PlaylistExportTrack {
    pub(crate) position: usize,
    pub(crate) file: String,
    #[serde(default)]
    pub(crate) original_path: String,
    #[serde(default)]
    pub(crate) key: String,
    #[serde(default)]
    pub(crate) file_hash: String,
    #[serde(default)]
    pub(crate) title: Option<String>,
    #[serde(default)]
    pub(crate) artist: Option<String>,
    #[serde(default)]
    pub(crate) album: Option<String>,
    #[serde(default)]
    pub(crate) year: Option<i32>,
    #[serde(default)]
    pub(crate) genre: Option<String>,
    #[serde(default)]
    pub(crate) track_number: Option<u32>,
    #[serde(default)]
    pub(crate) lyrics: Option<String>,
    #[serde(default)]
    pub(crate) curator_note: Option<String>,
}

pub struct PlaylistExportService;

impl PlaylistExportService {
    pub fn default_file_name(playlist: &Playlist) -> String {
        format!(
            "{}.birdplaylist.zip",
            Self::safe_file_stem(
                playlist
                    .get_name()
                    .as_deref()
                    .unwrap_or("Untitled Playlist")
            )
        )
    }

    pub fn export_playlist(
        playlist: &Playlist,
        output_path: &Path,
    ) -> Result<PlaylistExportResult, String> {
        match Self::write_playlist_archive(playlist, output_path) {
            Ok(result) => Ok(result),
            Err(err) => {
                let _ = fs::remove_file(output_path);
                Err(err)
            }
        }
    }

    fn write_playlist_archive(
        playlist: &Playlist,
        output_path: &Path,
    ) -> Result<PlaylistExportResult, String> {
        if playlist.tracks.is_empty() {
            return Err("Playlist has no tracks to export.".to_string());
        }

        let file = File::create(output_path)
            .map_err(|err| format!("Failed to create export file: {}", err))?;
        let writer = BufWriter::new(file);
        let mut zip = ZipWriter::new(writer);
        let options = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);

        let mut cover_archive_path = None;
        if let Some((mime_type, bytes)) = playlist.cover() {
            let extension = match mime_type {
                "image/png" => "png",
                "image/webp" => "webp",
                "image/gif" => "gif",
                _ => "jpg",
            };
            let archive_path = format!("cover/cover.{extension}");
            zip.start_file(&archive_path, options)
                .map_err(|err| format!("Failed to add playlist cover: {err}"))?;
            zip.write_all(bytes)
                .map_err(|err| format!("Failed to write playlist cover: {err}"))?;
            cover_archive_path = Some(archive_path);
        }

        let mut manifest_tracks = Vec::new();

        for (idx, track) in playlist.tracks.iter().enumerate() {
            let source_path = track.path();
            if !source_path.is_file() {
                return Err(format!(
                    "Track file was not found: {}",
                    source_path.display()
                ));
            }

            let archive_path = format!(
                "tracks/{:03}-{}",
                idx + 1,
                Self::safe_file_name(&source_path, idx + 1)
            );

            zip.start_file(&archive_path, options)
                .map_err(|err| format!("Failed to add track to export: {}", err))?;
            let mut source = File::open(&source_path)
                .map_err(|err| format!("Failed to read {}: {}", source_path.display(), err))?;
            std::io::copy(&mut source, &mut zip)
                .map_err(|err| format!("Failed to copy {}: {}", source_path.display(), err))?;

            manifest_tracks.push(PlaylistExportTrack {
                position: idx + 1,
                file: archive_path,
                original_path: source_path.to_string_lossy().to_string(),
                key: track.key(),
                file_hash: track.file_hash().to_string(),
                title: track.title(),
                artist: track.artist(),
                album: track.album(),
                year: track.year(),
                genre: track.genre(),
                track_number: track.track_number(),
                lyrics: track.lyrics(),
                curator_note: playlist.track_note(idx).map(str::to_string),
            });
        }

        let manifest = PlaylistExportManifest {
            format: "bird-player-playlist".to_string(),
            format_version: 2,
            name: playlist.get_name(),
            description: playlist.description(),
            curator: playlist.curator(),
            booklet: playlist.booklet(),
            cover: cover_archive_path,
            cover_mime_type: playlist.cover().map(|(mime_type, _)| mime_type.to_string()),
            avatar: None,
            created_at: playlist.created_at(),
            updated_at: playlist.updated_at(),
            tracks: manifest_tracks,
        };

        zip.start_file("playlist.json", options)
            .map_err(|err| format!("Failed to add playlist metadata: {}", err))?;
        let manifest_json = serde_json::to_vec_pretty(&manifest)
            .map_err(|err| format!("Failed to serialize playlist metadata: {}", err))?;
        zip.write_all(&manifest_json)
            .map_err(|err| format!("Failed to write playlist metadata: {}", err))?;
        let mut buffered_writer = zip
            .finish()
            .map_err(|err| format!("Failed to finish export zip: {}", err))?;
        buffered_writer
            .flush()
            .map_err(|err| format!("Failed to flush export file: {}", err))?;

        Ok(PlaylistExportResult {
            output_path: output_path.to_path_buf(),
            track_count: playlist.tracks.len(),
        })
    }

    fn safe_file_stem(name: &str) -> String {
        let sanitized = name
            .chars()
            .map(|ch| match ch {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
                ch if ch.is_control() => '-',
                ch => ch,
            })
            .collect::<String>();
        let trimmed = sanitized.trim().trim_matches('.').to_string();
        if trimmed.is_empty() {
            "Untitled Playlist".to_string()
        } else {
            trimmed
        }
    }

    fn safe_file_name(source_path: &Path, fallback_position: usize) -> String {
        source_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(Self::safe_file_stem)
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| format!("track-{}", fallback_position))
    }
}

#[cfg(test)]
mod tests {
    use super::PlaylistExportService;
    use crate::library::{LibraryItem, LibraryPathId};
    use std::io::Read;

    #[test]
    fn default_file_name_sanitizes_playlist_name() {
        let mut playlist = crate::playlist::Playlist::new();
        playlist.set_name("a/b:c*?".to_string());

        assert_eq!(
            PlaylistExportService::default_file_name(&playlist),
            "a-b-c--.birdplaylist.zip"
        );
    }

    #[test]
    fn version_two_archive_contains_booklet_cover_and_track_notes() {
        let test_dir =
            std::env::temp_dir().join(format!("bird-player-export-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&test_dir).unwrap();
        let track_path = test_dir.join("song.mp3");
        std::fs::write(&track_path, b"test audio bytes").unwrap();
        let archive_path = test_dir.join("playlist.birdplaylist.zip");

        let mut playlist = crate::playlist::Playlist::new();
        playlist.set_name("Night Drive".to_string());
        playlist.set_curator(Some("Bird".to_string()));
        playlist.set_booklet(Some("A small story.".to_string()));
        playlist.set_cover(Some("image/png".to_string()), Some(vec![1, 2, 3, 4]));
        playlist.add(
            LibraryItem::new(track_path, LibraryPathId::new(1)).set_title(Some("First Light")),
        );
        playlist.set_track_note(0, Some("Open the tape here.".to_string()));

        PlaylistExportService::export_playlist(&playlist, &archive_path).unwrap();

        let file = std::fs::File::open(&archive_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut manifest_json = String::new();
        archive
            .by_name("playlist.json")
            .unwrap()
            .read_to_string(&mut manifest_json)
            .unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&manifest_json).unwrap();
        assert_eq!(manifest["format_version"], 2);
        assert_eq!(manifest["curator"], "Bird");
        assert_eq!(manifest["booklet"], "A small story.");
        assert_eq!(manifest["cover"], "cover/cover.png");
        assert_eq!(manifest["tracks"][0]["curator_note"], "Open the tape here.");
        assert!(archive.by_name("cover/cover.png").is_ok());

        drop(archive);
        std::fs::remove_dir_all(test_dir).unwrap();
    }
}
