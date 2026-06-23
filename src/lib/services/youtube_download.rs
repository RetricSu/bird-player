use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;

const GITHUB_LATEST_YTDLP: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
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
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Music")
            .join("Bird Player Downloads")
    }

    fn remote_binary_name() -> &'static str {
        #[cfg(target_os = "macos")]
        {
            "yt-dlp_macos"
        }
        #[cfg(target_os = "windows")]
        {
            "yt-dlp.exe"
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            "yt-dlp"
        }
    }

    fn managed_bin_dir() -> PathBuf {
        #[cfg(target_os = "macos")]
        let base =
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"));
        #[cfg(target_os = "windows")]
        let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));

        base.unwrap_or_else(|| PathBuf::from("."))
            .join("bird-player")
            .join("bin")
    }

    fn managed_binary_path() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            Self::managed_bin_dir().join("yt-dlp").with_extension("exe")
        }
        #[cfg(not(target_os = "windows"))]
        {
            Self::managed_bin_dir().join("yt-dlp")
        }
    }

    fn is_executable(path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(path)
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        }

        #[cfg(windows)]
        {
            true
        }
    }

    fn version_check(path: &Path) -> bool {
        Command::new(path)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(target_os = "macos")]
    fn macos_candidates() -> Vec<PathBuf> {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let mut candidates = Vec::new();
        if let Some(h) = home {
            candidates.push(h.join(".local/bin/yt-dlp"));
        }
        candidates.push(PathBuf::from("/opt/homebrew/bin/yt-dlp"));
        candidates.push(PathBuf::from("/usr/local/bin/yt-dlp"));
        candidates
    }

    #[cfg(target_os = "linux")]
    fn linux_candidates() -> Vec<PathBuf> {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let mut candidates = Vec::new();
        if let Some(h) = home {
            candidates.push(h.join(".local/bin/yt-dlp"));
        }
        candidates.push(PathBuf::from("/usr/local/bin/yt-dlp"));
        candidates.push(PathBuf::from("/usr/bin/yt-dlp"));
        candidates
    }

    #[cfg(target_os = "windows")]
    fn windows_candidates() -> Vec<PathBuf> {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from);
        let mut candidates = Vec::new();
        if let Some(h) = home {
            candidates.push(h.join("AppData/Local/Programs/yt-dlp/yt-dlp.exe"));
            candidates.push(h.join(".local/bin/yt-dlp.exe"));
        }
        candidates.push(PathBuf::from(r"C:\Program Files\yt-dlp\yt-dlp.exe"));
        candidates
    }

    fn first_working_binary(candidates: &[PathBuf]) -> Option<PathBuf> {
        candidates
            .iter()
            .find(|p| Self::is_executable(p) && Self::version_check(p))
            .cloned()
    }

    fn find_existing_yt_dlp() -> Option<PathBuf> {
        let managed = Self::managed_binary_path();
        if Self::is_executable(&managed) && Self::version_check(&managed) {
            return Some(managed);
        }

        #[cfg(target_os = "macos")]
        let candidates = Self::macos_candidates();
        #[cfg(target_os = "linux")]
        let candidates = Self::linux_candidates();
        #[cfg(target_os = "windows")]
        let candidates = Self::windows_candidates();
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        let candidates: Vec<PathBuf> = Vec::new();

        if let Some(path) = Self::first_working_binary(&candidates) {
            return Some(path);
        }

        let path_only = PathBuf::from("yt-dlp");
        if Self::version_check(&path_only) {
            return Some(path_only);
        }

        None
    }

    fn download_managed_yt_dlp() -> Result<PathBuf, String> {
        let bin_dir = Self::managed_bin_dir();
        std::fs::create_dir_all(&bin_dir)
            .map_err(|e| format!("Failed to create yt-dlp directory: {}", e))?;

        let target_path = Self::managed_binary_path();
        let url = format!("{}/{}", GITHUB_LATEST_YTDLP, Self::remote_binary_name());

        let response = ureq::get(&url)
            .call()
            .map_err(|e| format!("Failed to download yt-dlp: {}", e))?;

        let mut reader = response.into_reader();
        let mut file = std::fs::File::create(&target_path)
            .map_err(|e| format!("Failed to create yt-dlp file: {}", e))?;
        std::io::copy(&mut reader, &mut file)
            .map_err(|e| format!("Failed to write yt-dlp file: {}", e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&target_path)
                .map_err(|e| format!("Failed to read yt-dlp permissions: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&target_path, perms)
                .map_err(|e| format!("Failed to make yt-dlp executable: {}", e))?;
        }

        if !Self::version_check(&target_path) {
            let _ = std::fs::remove_file(&target_path);
            return Err("Downloaded yt-dlp does not work".to_string());
        }

        Ok(target_path)
    }

    fn shell_path() -> Option<String> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let output = Command::new(&shell)
            .arg("-l")
            .arg("-c")
            .arg("printf '%s' \"$PATH\"")
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    }

    fn yt_dlp_command(path: &Path) -> Command {
        let mut cmd = Command::new(path);
        if let Some(user_path) = Self::shell_path() {
            cmd.env("PATH", user_path);
        }
        cmd
    }

    fn ensure_yt_dlp() -> Result<PathBuf, String> {
        if let Some(path) = Self::find_existing_yt_dlp() {
            Ok(path)
        } else {
            Self::download_managed_yt_dlp()
        }
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

        let yt_dlp = Self::ensure_yt_dlp()?;

        let output_template = output_dir
            .join("%(artist,creator,uploader|Unknown Artist)s - %(title)s [%(id)s].%(ext)s");

        let output = Self::yt_dlp_command(&yt_dlp)
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
            .arg("--")
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
