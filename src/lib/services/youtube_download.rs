use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub enum YoutubeDownloadEvent {
    Finished(Result<YoutubeDownloadResult, String>),
}

#[derive(Debug, Clone)]
pub struct YoutubeDownloadResult {
    pub output_dir: PathBuf,
    pub downloaded_files: Vec<PathBuf>,
}

pub struct YoutubeDownloadService;

impl YoutubeDownloadService {
    pub fn default_download_dir() -> PathBuf {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Music")
            .join("Bird Player Downloads")
    }

    pub fn download_authorized_audio(
        url: String,
        output_dir: PathBuf,
        event_tx: Sender<YoutubeDownloadEvent>,
    ) {
        std::thread::spawn(move || {
            let result = Self::run_download(&url, &output_dir);
            let _ = event_tx.send(YoutubeDownloadEvent::Finished(result));
        });
    }

    fn run_download(url: &str, output_dir: &Path) -> Result<YoutubeDownloadResult, String> {
        let url = url.trim();
        if url.is_empty() {
            return Err("URL is required".to_string());
        }

        if let Err(err) = std::fs::create_dir_all(output_dir) {
            return Err(format!("Failed to create output folder: {}", err));
        }

        if Command::new("yt-dlp").arg("--version").output().is_err() {
            return Err(
                "yt-dlp was not found. Install yt-dlp and make sure it is available in PATH."
                    .to_string(),
            );
        }

        let output_template = output_dir
            .join("%(artist,creator,uploader|Unknown Artist)s - %(title)s [%(id)s].%(ext)s");

        let output = Command::new("yt-dlp")
            .arg("--extract-audio")
            .arg("--audio-format")
            .arg("mp3")
            .arg("--audio-quality")
            .arg("0")
            .arg("--embed-metadata")
            .arg("--embed-thumbnail")
            .arg("--restrict-filenames")
            .arg("--print")
            .arg("after_move:filepath")
            .arg("--output")
            .arg(output_template)
            .arg(url)
            .output()
            .map_err(|err| format!("Failed to start yt-dlp: {}", err))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                format!("yt-dlp failed with status {}", output.status)
            } else {
                stderr
            });
        }

        let downloaded_files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(PathBuf::from)
            .filter(|path| path.exists())
            .collect::<Vec<_>>();

        Ok(YoutubeDownloadResult {
            output_dir: output_dir.to_path_buf(),
            downloaded_files,
        })
    }
}
