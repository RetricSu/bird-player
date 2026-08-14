use crate::timed_text::{parse_lrc, TimedTextCue};
use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::AudioFile;
use lofty::id3::v2::{
    BinaryFrame, Frame, FrameId, SyncTextContentType, SynchronizedTextFrame, TimestampFormat,
};
use lofty::mpeg::MpegFile;
use lofty::TextEncoding;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

const SUBTITLE_DESCRIPTOR_PREFIX: &str = "bird-player:subtitle:v1:";
const SYLT_FRAME_ID: FrameId<'static> = FrameId::Valid(Cow::Borrowed("SYLT"));

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubtitleTrack {
    pub language: String,
    pub source: String,
    pub cues: Vec<TimedTextCue>,
}

impl SubtitleTrack {
    pub fn from_lrc(language: impl Into<String>, source: impl Into<String>, content: &str) -> Self {
        Self {
            language: language.into(),
            source: source.into(),
            cues: parse_lrc(content),
        }
    }

    pub fn current_cue_index(&self, timestamp_ms: u64) -> Option<usize> {
        self.cues
            .iter()
            .position(|cue| cue.is_active_at(timestamp_ms))
    }
}

pub struct SubtitleService;

impl SubtitleService {
    pub fn read_from_audio(path: &Path) -> Result<Option<SubtitleTrack>, String> {
        if !is_mp3(path) {
            return Ok(None);
        }

        let mut file = OpenOptions::new()
            .read(true)
            .open(path)
            .map_err(|err| format!("Failed to open audio file: {}", err))?;
        let mpeg = <MpegFile as AudioFile>::read_from(&mut file, ParseOptions::default())
            .map_err(|err| format!("Failed to read MP3 metadata: {}", err))?;
        let Some(tag) = mpeg.id3v2() else {
            return Ok(None);
        };

        for frame in tag {
            let Frame::Binary(binary) = frame else {
                continue;
            };
            if binary.id().as_str() != "SYLT" {
                continue;
            }

            let Ok(synchronized) = SynchronizedTextFrame::parse(&binary.data, binary.flags())
            else {
                continue;
            };
            let Some(descriptor) = synchronized.description.as_deref() else {
                continue;
            };
            let Some((language, source)) = parse_descriptor(descriptor) else {
                continue;
            };

            let mut cues = synchronized
                .content
                .into_iter()
                .map(|(timestamp, text)| TimedTextCue {
                    text,
                    start_time_ms: Some(u64::from(timestamp)),
                    end_time_ms: None,
                })
                .collect::<Vec<_>>();
            for index in 0..cues.len().saturating_sub(1) {
                cues[index].end_time_ms = cues[index + 1].start_time_ms;
            }

            return Ok(Some(SubtitleTrack {
                language,
                source,
                cues,
            }));
        }

        Ok(None)
    }

    pub fn write_to_mp3(path: &Path, track: &SubtitleTrack) -> Result<(), String> {
        if !is_mp3(path) {
            return Err("Timed subtitles can currently be embedded only in MP3 files".to_string());
        }
        if track.cues.is_empty() {
            return Err("Subtitle track has no timed cues".to_string());
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|err| format!("Failed to open MP3 for subtitle writing: {}", err))?;
        let mut mpeg = <MpegFile as AudioFile>::read_from(&mut file, ParseOptions::default())
            .map_err(|err| format!("Failed to read MP3 metadata: {}", err))?;
        let mut tag = mpeg.id3v2_mut().map(std::mem::take).unwrap_or_default();

        tag.retain(|frame| !is_bird_player_subtitle_frame(frame));

        let synchronized = subtitle_to_sylt(track)?;
        let binary = BinaryFrame::new(
            SYLT_FRAME_ID,
            synchronized
                .as_bytes()
                .map_err(|err| format!("Failed to encode synchronized subtitles: {}", err))?,
        );
        tag.insert(Frame::Binary(binary));
        mpeg.set_id3v2(tag);
        mpeg.save_to(&mut file, WriteOptions::default())
            .map_err(|err| format!("Failed to write subtitles to MP3: {}", err))
    }

    pub fn embed_best_sidecar(
        audio_path: &Path,
        language_preferences: &[String],
    ) -> Result<bool, String> {
        let sidecars = subtitle_sidecars(audio_path)?;
        let Some((sidecar, language)) = choose_sidecar(sidecars, language_preferences) else {
            return Ok(false);
        };

        let content = std::fs::read_to_string(&sidecar).map_err(|err| {
            format!(
                "Failed to read subtitle file '{}': {}",
                sidecar.display(),
                err
            )
        })?;
        let source = youtube_id_from_audio_path(audio_path)
            .map(|id| format!("youtube:{}", id))
            .unwrap_or_else(|| "youtube".to_string());
        let track = SubtitleTrack::from_lrc(language, source, &content);
        Self::write_to_mp3(audio_path, &track)?;

        for (path, _) in subtitle_sidecars(audio_path)? {
            if let Err(err) = std::fs::remove_file(&path) {
                tracing::warn!(
                    "Failed to remove embedded subtitle sidecar '{}': {}",
                    path.display(),
                    err
                );
            }
        }

        Ok(true)
    }
}

fn subtitle_to_sylt(track: &SubtitleTrack) -> Result<SynchronizedTextFrame<'static>, String> {
    let content = track
        .cues
        .iter()
        .filter_map(|cue| {
            cue.start_time_ms.map(|timestamp| {
                (
                    u32::try_from(timestamp).unwrap_or(u32::MAX),
                    cue.text.clone(),
                )
            })
        })
        .collect::<Vec<_>>();
    if content.is_empty() {
        return Err("Subtitle track has no timed cues".to_string());
    }

    Ok(SynchronizedTextFrame::new(
        TextEncoding::UTF8,
        iso_639_2(&track.language),
        TimestampFormat::MS,
        SyncTextContentType::TextTranscription,
        Some(format!(
            "{}{}:{}",
            SUBTITLE_DESCRIPTOR_PREFIX, track.language, track.source
        )),
        content,
    ))
}

fn is_bird_player_subtitle_frame(frame: &Frame<'_>) -> bool {
    let Frame::Binary(binary) = frame else {
        return false;
    };
    if binary.id().as_str() != "SYLT" {
        return false;
    }

    SynchronizedTextFrame::parse(&binary.data, binary.flags())
        .ok()
        .and_then(|frame| frame.description)
        .is_some_and(|description| description.starts_with(SUBTITLE_DESCRIPTOR_PREFIX))
}

fn parse_descriptor(descriptor: &str) -> Option<(String, String)> {
    let remainder = descriptor.strip_prefix(SUBTITLE_DESCRIPTOR_PREFIX)?;
    let (language, source) = remainder.split_once(':')?;
    Some((language.to_string(), source.to_string()))
}

fn iso_639_2(language: &str) -> [u8; 3] {
    let normalized = language.to_ascii_lowercase();
    if normalized.starts_with("zh") {
        *b"zho"
    } else if normalized.starts_with("en") {
        *b"eng"
    } else if normalized.starts_with("ja") {
        *b"jpn"
    } else if normalized.starts_with("ko") {
        *b"kor"
    } else if normalized.starts_with("es") {
        *b"spa"
    } else if normalized.starts_with("fr") {
        *b"fra"
    } else if normalized.starts_with("de") {
        *b"deu"
    } else {
        *b"und"
    }
}

fn is_mp3(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("mp3"))
}

fn subtitle_sidecars(audio_path: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    let Some(parent) = audio_path.parent() else {
        return Ok(Vec::new());
    };
    let Some(stem) = audio_path.file_stem().and_then(|stem| stem.to_str()) else {
        return Ok(Vec::new());
    };
    let prefix = format!("{}.", stem);
    let mut sidecars = Vec::new();

    let entries = std::fs::read_dir(parent)
        .map_err(|err| format!("Failed to scan subtitle files: {}", err))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(language_with_extension) = file_name.strip_prefix(&prefix) else {
            continue;
        };
        let Some(language) = language_with_extension.strip_suffix(".lrc") else {
            continue;
        };
        if !language.is_empty() {
            let language = language.to_string();
            sidecars.push((path, language));
        }
    }

    Ok(sidecars)
}

fn choose_sidecar(
    mut sidecars: Vec<(PathBuf, String)>,
    preferences: &[String],
) -> Option<(PathBuf, String)> {
    sidecars.sort_by_key(|(_, language)| language_rank(language, preferences));
    sidecars.into_iter().next()
}

fn language_rank(language: &str, preferences: &[String]) -> usize {
    let language = language.to_ascii_lowercase();
    preferences
        .iter()
        .enumerate()
        .filter_map(|(index, preference)| {
            let preference = preference.trim_end_matches(".*").to_ascii_lowercase();
            if language == preference {
                Some(index * 2)
            } else if language.starts_with(&format!("{}-", preference)) {
                Some(index * 2 + 1)
            } else {
                None
            }
        })
        .min()
        .unwrap_or(preferences.len() * 2)
}

fn youtube_id_from_audio_path(path: &Path) -> Option<&str> {
    let stem = path.file_stem()?.to_str()?;
    let opening = stem.rfind('[')?;
    let id = stem.get(opening + 1..)?.strip_suffix(']')?;
    (!id.is_empty()).then_some(id)
}

#[cfg(test)]
mod tests {
    use super::{
        choose_sidecar, parse_descriptor, subtitle_to_sylt, SubtitleService, SubtitleTrack,
        SUBTITLE_DESCRIPTOR_PREFIX,
    };
    use crate::lyrics::{Lyrics, LyricsService};
    use lofty::id3::v2::{FrameFlags, SyncTextContentType, SynchronizedTextFrame};
    use std::path::PathBuf;

    #[test]
    fn subtitle_sylt_round_trip_preserves_timing_and_descriptor() {
        let track = SubtitleTrack::from_lrc(
            "zh-Hans",
            "youtube:abc123",
            "[00:01.00]你好\n[00:02.50]世界",
        );

        let encoded = subtitle_to_sylt(&track).expect("subtitle should encode");
        let decoded =
            SynchronizedTextFrame::parse(&encoded.as_bytes().unwrap(), FrameFlags::default())
                .expect("subtitle should decode");

        assert_eq!(decoded.content_type, SyncTextContentType::TextTranscription);
        assert_eq!(decoded.content[0], (1_000, "你好".to_string()));
        assert_eq!(
            decoded.description.as_deref(),
            Some("bird-player:subtitle:v1:zh-Hans:youtube:abc123")
        );
    }

    #[test]
    fn descriptor_parsing_keeps_source_suffix() {
        assert_eq!(
            parse_descriptor(&format!("{}en:youtube:abc123", SUBTITLE_DESCRIPTOR_PREFIX)),
            Some(("en".to_string(), "youtube:abc123".to_string()))
        );
    }

    #[test]
    fn sidecar_choice_follows_language_preference() {
        let chosen = choose_sidecar(
            vec![
                (PathBuf::from("track.en.lrc"), "en".to_string()),
                (PathBuf::from("track.zh-Hans.lrc"), "zh-Hans".to_string()),
            ],
            &["zh-Hans".to_string(), "en".to_string()],
        )
        .unwrap();

        assert_eq!(chosen.1, "zh-Hans");
    }

    #[test]
    fn mp3_round_trip_keeps_lyrics_and_embedded_subtitles_separate() {
        if std::process::Command::new("ffmpeg")
            .arg("-version")
            .output()
            .is_err()
        {
            return;
        }

        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "bird-player-subtitle-round-trip-{}-{}.mp3",
            std::process::id(),
            unique
        ));
        let generated = std::process::Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "anullsrc=r=44100:cl=stereo",
                "-t",
                "0.1",
                "-q:a",
                "9",
                "-y",
            ])
            .arg(&path)
            .status()
            .expect("ffmpeg should start");
        assert!(generated.success());

        let lyrics = Lyrics {
            id: 0,
            name: "Test Lyrics".to_string(),
            track_name: "Track".to_string(),
            artist_name: "Artist".to_string(),
            album_name: None,
            duration: None,
            instrumental: false,
            plain_lyrics: Some("Existing lyrics".to_string()),
            synced_lyrics: None,
            lines: Vec::new(),
        };
        LyricsService::write_lyrics_to_file(&path, &lyrics).expect("lyrics should write");

        let subtitles = SubtitleTrack::from_lrc("en", "youtube:test", "[00:00.00]Caption text");
        SubtitleService::write_to_mp3(&path, &subtitles).expect("subtitles should write");

        let loaded_subtitles = SubtitleService::read_from_audio(&path)
            .expect("subtitles should read")
            .expect("subtitle track should exist");
        let loaded_lyrics = LyricsService::read_lyrics_from_file(&path, "Artist", "Track")
            .expect("lyrics should remain");

        assert_eq!(loaded_subtitles.cues[0].text, "Caption text");
        assert_eq!(
            loaded_lyrics.plain_lyrics.as_deref(),
            Some("Existing lyrics")
        );

        let _ = std::fs::remove_file(path);
    }
}
