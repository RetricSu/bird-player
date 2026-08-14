use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum YoutubeDownloadEvent {
    Progress(f32),
    Finished(Result<YoutubeDownloadResult, String>),
    SearchFinished(Result<Vec<YoutubeSearchResult>, String>),
}

#[derive(Debug, Clone)]
pub struct YoutubeDownloadResult {
    pub output_dir: PathBuf,
    pub downloaded_files: Vec<PathBuf>,
    pub embedded_subtitle_count: usize,
    pub subtitle_warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct YoutubeDownloadOptions {
    pub include_playlist: bool,
    pub download_subtitles: bool,
    pub subtitle_languages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YoutubeSearchResult {
    pub title: String,
    pub channel: String,
    pub url: String,
    pub duration: Option<u64>,
    pub thumbnail_url: Option<String>,
}

pub struct YoutubeDownloadService;

const PROGRESS_PREFIX: &str = "BIRD_PLAYER_PROGRESS:";

impl YoutubeDownloadService {
    pub fn default_download_dir() -> PathBuf {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Music")
            .join("Bird Player Downloads")
    }

    pub fn download_authorized_audio(
        url: String,
        output_dir: PathBuf,
        options: YoutubeDownloadOptions,
        event_tx: Sender<YoutubeDownloadEvent>,
    ) {
        std::thread::spawn(move || {
            let result = Self::run_download(&url, &output_dir, &options, &event_tx);
            let _ = event_tx.send(YoutubeDownloadEvent::Finished(result));
        });
    }

    pub fn search_youtube(query: String, limit: usize, event_tx: Sender<YoutubeDownloadEvent>) {
        std::thread::spawn(move || {
            let result = Self::run_search(&query, limit);
            let _ = event_tx.send(YoutubeDownloadEvent::SearchFinished(result));
        });
    }

    fn run_search(query: &str, limit: usize) -> Result<Vec<YoutubeSearchResult>, String> {
        let query = query.trim();
        if query.is_empty() {
            return Err("Search query is required".to_string());
        }

        if Self::yt_dlp_command().arg("--version").output().is_err() {
            return Err(
                "yt-dlp was not found. Install yt-dlp and make sure it is available in PATH."
                    .to_string(),
            );
        }

        let search_target = format!("ytsearch{}:{}", limit.max(1), query);
        let output = Self::youtube_command()
            .arg("--dump-json")
            .arg("--flat-playlist")
            .arg("--no-warnings")
            .arg("--")
            .arg(search_target)
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

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(Self::parse_search_results(&stdout))
    }

    fn run_download(
        url: &str,
        output_dir: &Path,
        options: &YoutubeDownloadOptions,
        event_tx: &Sender<YoutubeDownloadEvent>,
    ) -> Result<YoutubeDownloadResult, String> {
        let url = url.trim();
        if url.is_empty() {
            return Err("URL is required".to_string());
        }
        let url = Self::playlist_safe_url(url, options.include_playlist);

        if let Err(err) = std::fs::create_dir_all(output_dir) {
            return Err(format!("Failed to create output folder: {}", err));
        }

        if Self::yt_dlp_command().arg("--version").output().is_err() {
            return Err(
                "yt-dlp was not found. Install yt-dlp and make sure it is available in PATH."
                    .to_string(),
            );
        }

        let output_template = output_dir
            .join("%(artist,creator,uploader|Unknown Artist)s - %(title)s [%(id)s].%(ext)s");

        let mut command = Self::youtube_command();
        command
            .arg("--color")
            .arg("never")
            .arg("--newline")
            .arg("--progress")
            .arg("--progress-delta")
            .arg("0.5")
            .arg("--progress-template")
            .arg(format!(
                "download:{}%(progress._percent_str)s",
                PROGRESS_PREFIX
            ))
            .arg("--extract-audio")
            .arg("--audio-format")
            .arg("mp3")
            .arg("--audio-quality")
            .arg("0")
            .arg("--embed-metadata")
            .arg("--embed-thumbnail")
            .arg("--restrict-filenames");

        Self::add_subtitle_download_args(&mut command, options);

        if options.include_playlist {
            command.arg("--yes-playlist");
        } else {
            command.arg("--no-playlist");
        }

        let mut child = command
            .arg("--print")
            .arg("after_move:filepath")
            .arg("--output")
            .arg(output_template)
            .arg("--")
            .arg(url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| format!("Failed to start yt-dlp: {}", err))?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (line_tx, line_rx) = std::sync::mpsc::channel();
        let stdout_handle = stdout.map(|stream| Self::read_lines(stream, line_tx.clone()));
        let stderr_handle = stderr.map(|stream| Self::read_lines(stream, line_tx));

        let mut downloaded_files = Vec::new();
        let mut output_lines = Vec::new();

        loop {
            if let Ok(line) = line_rx.recv_timeout(Duration::from_millis(100)) {
                Self::handle_download_output_line(
                    &line,
                    &mut downloaded_files,
                    &mut output_lines,
                    event_tx,
                );
                for line in line_rx.try_iter() {
                    Self::handle_download_output_line(
                        &line,
                        &mut downloaded_files,
                        &mut output_lines,
                        event_tx,
                    );
                }
            }

            if let Some(status) = child
                .try_wait()
                .map_err(|err| format!("Failed to wait for yt-dlp: {}", err))?
            {
                if let Some(handle) = stdout_handle {
                    let _ = handle.join();
                }
                if let Some(handle) = stderr_handle {
                    let _ = handle.join();
                }
                for line in line_rx.try_iter() {
                    Self::handle_download_output_line(
                        &line,
                        &mut downloaded_files,
                        &mut output_lines,
                        event_tx,
                    );
                }

                if !status.success() {
                    let details = output_lines
                        .iter()
                        .rev()
                        .take(8)
                        .cloned()
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>()
                        .join("\n");
                    return Err(if details.trim().is_empty() {
                        format!("yt-dlp failed with status {}", status)
                    } else {
                        details
                    });
                }
                break;
            }
        }

        downloaded_files.sort();
        downloaded_files.dedup();

        let (embedded_subtitle_count, subtitle_warnings) =
            Self::embed_downloaded_subtitles(&downloaded_files, options);

        Ok(YoutubeDownloadResult {
            output_dir: output_dir.to_path_buf(),
            downloaded_files,
            embedded_subtitle_count,
            subtitle_warnings,
        })
    }

    fn add_subtitle_download_args(command: &mut Command, options: &YoutubeDownloadOptions) {
        if !options.download_subtitles {
            return;
        }

        let languages = if options.subtitle_languages.is_empty() {
            "en,en-orig".to_string()
        } else {
            options.subtitle_languages.join(",")
        };
        command
            .arg("--write-subs")
            .arg("--write-auto-subs")
            .arg("--sub-langs")
            .arg(languages)
            .arg("--sub-format")
            .arg("vtt/best")
            .arg("--convert-subs")
            .arg("lrc");
    }

    fn embed_downloaded_subtitles(
        downloaded_files: &[PathBuf],
        options: &YoutubeDownloadOptions,
    ) -> (usize, Vec<String>) {
        if !options.download_subtitles {
            return (0, Vec::new());
        }

        let mut embedded_count = 0;
        let mut warnings = Vec::new();
        for audio_path in downloaded_files {
            match crate::subtitles::SubtitleService::embed_best_sidecar(
                audio_path,
                &options.subtitle_languages,
            ) {
                Ok(true) => embedded_count += 1,
                Ok(false) => {}
                Err(err) => {
                    tracing::warn!(
                        "Failed to embed downloaded subtitles into '{}': {}",
                        audio_path.display(),
                        err
                    );
                    warnings.push(format!("{}: {}", audio_path.display(), err));
                }
            }
        }

        (embedded_count, warnings)
    }

    fn yt_dlp_command() -> Command {
        let mut command = Command::new("yt-dlp");
        command.env("PATH", Self::app_runtime_path());
        command
    }

    fn youtube_command() -> Command {
        let runtime_path = Self::app_runtime_path();
        let mut command = Command::new("yt-dlp");
        command.env("PATH", &runtime_path);

        let js_runtime = Self::supported_js_runtime(&runtime_path);
        let browser = Self::browser_cookie_source();
        Self::add_youtube_access_args(&mut command, js_runtime.as_deref(), browser.as_deref());
        command
    }

    fn add_youtube_access_args(
        command: &mut Command,
        js_runtime: Option<&str>,
        browser: Option<&str>,
    ) {
        if let Some(js_runtime) = js_runtime {
            command
                .arg("--js-runtimes")
                .arg(js_runtime)
                .arg("--remote-components")
                .arg("ejs:github");
        }

        if let Some(browser) = browser {
            command.arg("--cookies-from-browser").arg(browser);
        }
    }

    fn supported_js_runtime(runtime_path: &OsString) -> Option<String> {
        let paths = std::env::split_paths(runtime_path).collect::<Vec<_>>();
        [
            ("deno", 2_u64, 3_u64),
            ("node", 22_u64, 0_u64),
            ("qjs", 2023_u64, 12_u64),
        ]
        .into_iter()
        .find_map(|(name, minimum_major, minimum_minor)| {
            let executable = Self::find_executable(name, &paths)?;
            let output = Command::new(&executable).arg("--version").output().ok()?;
            if !output.status.success() {
                return None;
            }

            let version_text = String::from_utf8_lossy(&output.stdout);
            Self::version_at_least(&version_text, minimum_major, minimum_minor)
                .then(|| format!("{}:{}", name, executable.display()))
        })
    }

    fn find_executable(name: &str, paths: &[PathBuf]) -> Option<PathBuf> {
        paths
            .iter()
            .map(|path| path.join(name))
            .find(|path| path.is_file())
    }

    fn version_at_least(version_text: &str, minimum_major: u64, minimum_minor: u64) -> bool {
        let version = version_text
            .split_whitespace()
            .find(|part| part.chars().next().is_some_and(|ch| ch.is_ascii_digit()))
            .unwrap_or(version_text);
        let mut numbers = version
            .trim_start_matches('v')
            .split(|ch: char| !ch.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.parse::<u64>().ok());
        let major = numbers.next().unwrap_or_default();
        let minor = numbers.next().unwrap_or_default();

        (major, minor) >= (minimum_major, minimum_minor)
    }

    fn browser_cookie_source() -> Option<String> {
        if let Ok(browser) = std::env::var("BIRD_PLAYER_YTDLP_BROWSER") {
            let browser = browser.trim();
            if matches!(
                browser,
                "brave"
                    | "chrome"
                    | "chromium"
                    | "edge"
                    | "firefox"
                    | "opera"
                    | "safari"
                    | "vivaldi"
                    | "whale"
            ) {
                return Some(browser.to_string());
            }
        }

        let home = PathBuf::from(std::env::var_os("HOME")?);
        [
            (
                "chrome",
                home.join("Library/Application Support/Google/Chrome"),
            ),
            (
                "brave",
                home.join("Library/Application Support/BraveSoftware/Brave-Browser"),
            ),
            (
                "firefox",
                home.join("Library/Application Support/Firefox/Profiles"),
            ),
            ("safari", home.join("Library/Cookies")),
            ("chrome", home.join(".config/google-chrome")),
            ("brave", home.join(".config/BraveSoftware/Brave-Browser")),
            ("firefox", home.join(".mozilla/firefox")),
        ]
        .into_iter()
        .find_map(|(browser, path)| path.exists().then(|| browser.to_string()))
    }

    fn app_runtime_path() -> OsString {
        let mut paths = std::env::var_os("PATH")
            .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
            .unwrap_or_default();

        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            Self::push_path_once(&mut paths, home.join(".local/bin"));
            Self::push_path_once(&mut paths, home.join("bin"));
            Self::push_path_once(&mut paths, home.join(".deno/bin"));
            Self::push_path_once(&mut paths, home.join(".volta/bin"));

            let nvm_versions = home.join(".nvm/versions/node");
            if let Ok(entries) = std::fs::read_dir(nvm_versions) {
                let mut node_bins = entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path().join("bin"))
                    .filter(|path| path.is_dir())
                    .collect::<Vec<_>>();
                node_bins.sort_by(|left, right| right.cmp(left));
                for node_bin in node_bins {
                    Self::push_path_once(&mut paths, node_bin);
                }
            }
        }

        for path in [
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin",
        ] {
            Self::push_path_once(&mut paths, PathBuf::from(path));
        }

        std::env::join_paths(paths).unwrap_or_else(|_| {
            OsString::from("/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin")
        })
    }

    fn push_path_once(paths: &mut Vec<PathBuf>, path: PathBuf) {
        if !paths.iter().any(|existing| existing == &path) {
            paths.push(path);
        }
    }

    fn read_lines<R: std::io::Read + Send + 'static>(
        stream: R,
        line_tx: std::sync::mpsc::Sender<String>,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            use std::io::BufRead;

            let mut reader = std::io::BufReader::new(stream);
            let mut buf = Vec::new();
            while let Ok(n) = reader.read_until(b'\n', &mut buf) {
                if n == 0 {
                    break;
                }
                if buf.ends_with(b"\n") {
                    buf.pop();
                    if buf.ends_with(b"\r") {
                        buf.pop();
                    }
                }
                let line = String::from_utf8_lossy(&buf).into_owned();
                let _ = line_tx.send(line);
                buf.clear();
            }
        })
    }

    fn handle_download_output_line(
        line: &str,
        downloaded_files: &mut Vec<PathBuf>,
        output_lines: &mut Vec<String>,
        event_tx: &Sender<YoutubeDownloadEvent>,
    ) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return;
        }

        let is_progress_line = if let Some(progress) =
            Self::parse_machine_progress(trimmed).or_else(|| Self::parse_progress(trimmed))
        {
            let _ = event_tx.send(YoutubeDownloadEvent::Progress(progress));
            true
        } else {
            false
        };

        let path = PathBuf::from(trimmed);
        if path.exists() {
            downloaded_files.push(path);
        } else if !is_progress_line {
            output_lines.push(trimmed.to_string());
        }
    }

    fn parse_machine_progress(line: &str) -> Option<f32> {
        let progress_text = line.strip_prefix(PROGRESS_PREFIX)?.trim();
        Self::parse_progress(progress_text)
    }

    fn parse_progress(line: &str) -> Option<f32> {
        let percent_pos = line.find('%')?;
        let before_percent = &line[..percent_pos];
        let start = before_percent
            .rfind(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
            .map_or(0, |idx| idx + 1);
        before_percent[start..]
            .parse::<f32>()
            .ok()
            .map(|percent| (percent / 100.0).clamp(0.0, 1.0))
    }

    fn playlist_safe_url(url: &str, include_playlist: bool) -> String {
        if include_playlist {
            return url.to_string();
        }

        let (url_without_fragment, fragment) = url
            .split_once('#')
            .map_or((url, None), |(url, fragment)| (url, Some(fragment)));

        let Some((base, query)) = url_without_fragment.split_once('?') else {
            return url.to_string();
        };

        let kept_params = query
            .split('&')
            .filter(|param| {
                let key = param.split_once('=').map_or(*param, |(key, _)| key);
                !matches!(key, "list" | "index" | "start_radio" | "pp")
            })
            .collect::<Vec<_>>();

        let mut rebuilt = if kept_params.is_empty() {
            base.to_string()
        } else {
            format!("{}?{}", base, kept_params.join("&"))
        };
        if let Some(fragment) = fragment {
            rebuilt.push('#');
            rebuilt.push_str(fragment);
        }
        rebuilt
    }

    fn parse_search_results(output: &str) -> Vec<YoutubeSearchResult> {
        output
            .lines()
            .filter_map(Self::parse_search_result_line)
            .collect()
    }

    fn parse_search_result_line(line: &str) -> Option<YoutubeSearchResult> {
        let value: serde_json::Value = serde_json::from_str(line).ok()?;
        let title = value.get("title")?.as_str()?.trim().to_string();
        if title.is_empty() {
            return None;
        }

        let channel = value
            .get("channel")
            .or_else(|| value.get("uploader"))
            .and_then(|value| value.as_str())
            .unwrap_or("YouTube")
            .trim()
            .to_string();

        let url = value
            .get("webpage_url")
            .or_else(|| value.get("url"))
            .and_then(|value| value.as_str())
            .map(|url| {
                if url.starts_with("http://") || url.starts_with("https://") {
                    url.to_string()
                } else {
                    format!("https://www.youtube.com/watch?v={}", url)
                }
            })
            .or_else(|| {
                value
                    .get("id")
                    .and_then(|value| value.as_str())
                    .map(|id| format!("https://www.youtube.com/watch?v={}", id))
            })?;

        Some(YoutubeSearchResult {
            title,
            channel,
            url,
            duration: value.get("duration").and_then(|value| value.as_u64()),
            thumbnail_url: Self::search_result_thumbnail_url(&value),
        })
    }

    fn search_result_thumbnail_url(value: &serde_json::Value) -> Option<String> {
        value
            .get("thumbnail")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|url| !url.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                value
                    .get("thumbnails")
                    .and_then(|value| value.as_array())
                    .and_then(|thumbnails| thumbnails.iter().rev().find_map(Self::thumbnail_url))
            })
    }

    fn thumbnail_url(value: &serde_json::Value) -> Option<String> {
        value
            .get("url")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|url| !url.is_empty())
            .map(ToOwned::to_owned)
    }
}

#[cfg(test)]
mod tests {
    use super::{YoutubeDownloadOptions, YoutubeDownloadService};

    #[test]
    fn playlist_safe_url_removes_playlist_params_by_default() {
        let url = "https://www.youtube.com/watch?v=P8jOQUsTU9o&list=RDGMEM6ijAnFTG9nX1G-kbWBUCJA&index=7&pp=8AUB";

        assert_eq!(
            YoutubeDownloadService::playlist_safe_url(url, false),
            "https://www.youtube.com/watch?v=P8jOQUsTU9o"
        );
    }

    #[test]
    fn playlist_safe_url_keeps_playlist_when_enabled() {
        let url =
            "https://www.youtube.com/watch?v=P8jOQUsTU9o&list=RDGMEM6ijAnFTG9nX1G-kbWBUCJA&index=7";

        assert_eq!(YoutubeDownloadService::playlist_safe_url(url, true), url);
    }

    #[test]
    fn parse_progress_reads_yt_dlp_percentages() {
        let progress = YoutubeDownloadService::parse_progress("[download]  42.5% of 4.00MiB")
            .expect("progress should parse");

        assert!((progress - 0.425).abs() < f32::EPSILON);
    }

    #[test]
    fn parse_machine_progress_requires_bird_player_prefix() {
        let progress =
            YoutubeDownloadService::parse_machine_progress("BIRD_PLAYER_PROGRESS:  42.5%")
                .expect("machine progress should parse");

        assert!((progress - 0.425).abs() < f32::EPSILON);
        assert!(YoutubeDownloadService::parse_machine_progress("[download] 42.5%").is_none());
    }

    #[test]
    fn app_runtime_path_includes_user_binary_dirs() {
        let path = YoutubeDownloadService::app_runtime_path();
        let paths = std::env::split_paths(&path).collect::<Vec<_>>();

        if let Some(home) = std::env::var_os("HOME") {
            let home = std::path::PathBuf::from(home);
            assert!(paths.contains(&home.join(".local/bin")));
            assert!(paths.contains(&home.join(".deno/bin")));
            assert!(paths.contains(&home.join(".volta/bin")));
        }
        assert!(paths.contains(&std::path::PathBuf::from("/opt/homebrew/bin")));
        assert!(paths.contains(&std::path::PathBuf::from("/usr/local/bin")));
    }

    #[test]
    fn supported_runtime_versions_are_parsed() {
        assert!(!YoutubeDownloadService::version_at_least(
            "deno 1.37.2",
            2,
            3
        ));
        assert!(YoutubeDownloadService::version_at_least("deno 2.3.0", 2, 3));
        assert!(YoutubeDownloadService::version_at_least("v24.8.0", 22, 0));
    }

    #[test]
    fn youtube_access_args_include_runtime_components_and_browser_cookies() {
        let mut command = std::process::Command::new("yt-dlp");
        YoutubeDownloadService::add_youtube_access_args(
            &mut command,
            Some("node:/example/node"),
            Some("chrome"),
        );
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(args
            .windows(2)
            .any(|args| args == ["--js-runtimes", "node:/example/node"]));
        assert!(args
            .windows(2)
            .any(|args| args == ["--remote-components", "ejs:github"]));
        assert!(args
            .windows(2)
            .any(|args| args == ["--cookies-from-browser", "chrome"]));
    }

    #[test]
    fn parse_search_results_reads_yt_dlp_json_lines() {
        let output = r#"{"id":"abc123","title":"Bird Song","channel":"Bird Channel","duration":245,"thumbnails":[{"url":"https://img.example/small.jpg"},{"url":"https://img.example/large.jpg"}]}
{"title":"No URL"}
{"webpage_url":"https://www.youtube.com/watch?v=def456","title":"Second Song","uploader":"Uploader","duration":60}"#;

        let results = YoutubeDownloadService::parse_search_results(output);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "Bird Song");
        assert_eq!(results[0].channel, "Bird Channel");
        assert_eq!(results[0].url, "https://www.youtube.com/watch?v=abc123");
        assert_eq!(results[0].duration, Some(245));
        assert_eq!(
            results[0].thumbnail_url.as_deref(),
            Some("https://img.example/large.jpg")
        );
        assert_eq!(results[1].url, "https://www.youtube.com/watch?v=def456");
    }

    #[test]
    fn subtitle_download_args_include_manual_auto_and_language_preferences() {
        let mut command = std::process::Command::new("yt-dlp");
        YoutubeDownloadService::add_subtitle_download_args(
            &mut command,
            &YoutubeDownloadOptions {
                include_playlist: false,
                download_subtitles: true,
                subtitle_languages: vec!["zh-Hans".to_string(), "en".to_string()],
            },
        );
        let args = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(args
            .windows(2)
            .any(|args| args == ["--sub-langs", "zh-Hans,en"]));
        assert!(args.contains(&"--write-subs".to_string()));
        assert!(args.contains(&"--write-auto-subs".to_string()));
        assert!(args
            .windows(2)
            .any(|args| args == ["--convert-subs", "lrc"]));
    }
}
