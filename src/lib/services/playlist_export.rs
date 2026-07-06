use crate::playlist::Playlist;
use serde::Serialize;
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

#[derive(Debug, Serialize)]
struct PlaylistExportManifest {
    format: String,
    format_version: u32,
    name: Option<String>,
    description: Option<String>,
    avatar: Option<String>,
    created_at: i64,
    updated_at: i64,
    tracks: Vec<PlaylistExportTrack>,
}

#[derive(Debug, Serialize)]
struct PlaylistExportTrack {
    position: usize,
    file: String,
    original_path: String,
    key: String,
    file_hash: String,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    year: Option<i32>,
    genre: Option<String>,
    track_number: Option<u32>,
    lyrics: Option<String>,
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
            });
        }

        let manifest = PlaylistExportManifest {
            format: "bird-player-playlist".to_string(),
            format_version: 1,
            name: playlist.get_name(),
            description: playlist.description(),
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

    #[test]
    fn default_file_name_sanitizes_playlist_name() {
        let mut playlist = crate::playlist::Playlist::new();
        playlist.set_name("a/b:c*?".to_string());

        assert_eq!(
            PlaylistExportService::default_file_name(&playlist),
            "a-b-c--.birdplaylist.zip"
        );
    }
}
