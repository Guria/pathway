use super::{BrowserInfo, BrowserKind};
use crate::browser::channels::{BrowserChannel, ChromiumChannel, FirefoxChannel, OperaChannel};
use crate::filesystem::FileSystem;
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};

use super::{LaunchCommand, LaunchOutcome, LaunchTarget, SystemDefaultBrowser};
use std::process::{Command, Stdio};
use thiserror::Error;
use tracing::debug;

fn fs_is_file<F: FileSystem>(fs: &F, path: &Path) -> bool {
    fs.metadata(path)
        .map(|meta| meta.is_file())
        .unwrap_or(false)
}

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("Browser executable missing: {0}")]
    MissingExecutable(String),
    #[error("No URLs provided to launch")]
    NoUrls,
    #[error("Failed to launch browser: {source}")]
    Spawn {
        #[from]
        source: std::io::Error,
    },
}

pub fn launch(target: LaunchTarget<'_>, urls: &[String]) -> Result<LaunchOutcome, LaunchError> {
    launch_with_profile(target, urls, None, None)
}
pub fn launch_with_profile(
    target: LaunchTarget<'_>,
    urls: &[String],
    profile_opts: Option<&crate::profile::ProfileOptions>,
    window_opts: Option<&crate::profile::WindowOptions>,
) -> Result<LaunchOutcome, LaunchError> {
    if urls.is_empty() {
        return Err(LaunchError::NoUrls);
    }

    match target {
        LaunchTarget::Browser(info) => {
            let (program, resolved_args, urls_consumed) = prepare_launch_command(info, urls)?;

            let mut command = Command::new(&program);

            let mut profile_args = Vec::new();
            let mut has_profile_args = false;
            if let (Some(profile_opts), Some(window_opts)) = (profile_opts, window_opts) {
                profile_args = crate::profile::ProfileManager::generate_profile_args(
                    info,
                    profile_opts,
                    window_opts,
                );
                has_profile_args = !profile_args.is_empty();
            }

            command.args(&resolved_args);
            if has_profile_args {
                command.args(&profile_args);
            }
            if !urls_consumed {
                command.args(urls);
            }

            command.stdin(Stdio::null());
            command.stdout(Stdio::null());
            command.stderr(Stdio::null());

            let all_args: Vec<String> = command
                .get_args()
                .map(|s| s.to_string_lossy().to_string())
                .collect();

            let log_message = if has_profile_args {
                "Launching browser with profile"
            } else {
                "Launching browser"
            };
            debug!(program = %program.display(), args = ?all_args, "{}", log_message);
            command.spawn()?;

            let cmd = LaunchCommand {
                program: program.clone(),
                args: all_args.clone(),
                display: format!("{} {}", program.display(), all_args.join(" ")),
                is_system_default: false,
            };

            Ok(LaunchOutcome {
                browser: Some(info.clone()),
                system_default: None,
                command: cmd,
            })
        }
        LaunchTarget::SystemDefault => {
            let mut command = Command::new("xdg-open");
            command.args(urls);
            command.stdin(Stdio::null());
            command.stdout(Stdio::null());
            command.stderr(Stdio::null());

            let all_args: Vec<String> = command
                .get_args()
                .map(|s| s.to_string_lossy().to_string())
                .collect();
            debug!(program = "xdg-open", args = ?all_args, "Launching system default browser");
            command.spawn()?;

            let cmd = LaunchCommand {
                program: PathBuf::from("xdg-open"),
                args: all_args.clone(),
                display: format!("xdg-open {}", all_args.join(" ")),
                is_system_default: true,
            };

            Ok(LaunchOutcome {
                browser: None,
                system_default: system_default_browser_with_fs(&crate::filesystem::RealFileSystem),
                command: cmd,
            })
        }
    }
}
pub fn system_default_browser_with_fs<F: FileSystem>(fs: &F) -> Option<SystemDefaultBrowser> {
    let desktop_id = detect_default_desktop_entry(fs)?;
    let desktop_path = resolve_desktop_entry_path(fs, &desktop_id)?;
    let content = fs.read_to_string(&desktop_path).ok()?;

    if let Some(info) = create_browser_info(&desktop_path, &content) {
        let path = info.launch_path().to_path_buf();
        return Some(SystemDefaultBrowser {
            identifier: desktop_id,
            display_name: info.display_name,
            kind: Some(info.kind),
            path: Some(path),
        });
    }

    let display_name = get_desktop_entry_value(&content, "Name")
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            desktop_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("system-default")
                .to_string()
        });

    Some(SystemDefaultBrowser {
        identifier: desktop_id,
        display_name,
        kind: None,
        path: Some(desktop_path),
    })
}

pub fn detect_browsers<F: FileSystem>(fs: &F) -> Vec<BrowserInfo> {
    let mut browsers = Vec::new();
    let mut processed_files = HashSet::new();

    for dir in desktop_file_dirs() {
        if !fs.is_dir(&dir) {
            continue;
        }
        // fs::walk_dir is not available on the FileSystem trait, so we use std::fs here.
        // The file reading itself will still use the trait for testability.
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    if let Ok(canonical_path) = fs.canonicalize(&path) {
                        if processed_files.contains(&canonical_path) {
                            continue;
                        }

                        if let Ok(content) = fs.read_to_string(&path) {
                            if is_web_browser(&content) {
                                if let Some(browser_info) = create_browser_info(&path, &content) {
                                    browsers.push(browser_info);
                                    processed_files.insert(canonical_path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    browsers
}

fn desktop_file_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        PathBuf::from("/var/lib/snapd/desktop/applications"),
    ];
    if let Ok(home) = env::var("HOME") {
        dirs.push(Path::new(&home).join(".local/share/applications"));
        dirs.push(Path::new(&home).join(".local/share/flatpak/exports/share/applications"));
    }
    dirs
}

fn get_desktop_entry_value<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    content.lines().find_map(|line| {
        let (lhs, rhs) = line.split_once('=')?;
        if lhs.trim() == key {
            Some(rhs.trim())
        } else {
            None
        }
    })
}

fn is_web_browser(content: &str) -> bool {
    if let Some(mime_type) = get_desktop_entry_value(content, "MimeType") {
        return mime_type.split(';').any(|t| t == "x-scheme-handler/https");
    }
    false
}

fn create_browser_info(path: &Path, content: &str) -> Option<BrowserInfo> {
    let (kind, channel) =
        parse_desktop_file_name(path.to_str()?).or_else(|| infer_kind_from_entry(path, content))?;

    let display_name = get_desktop_entry_value(content, "Name")
        .map(|s| s.to_string())
        .unwrap_or_else(|| kind.canonical_name().to_string());

    let exec_value = get_desktop_entry_value(content, "Exec")?;
    let executable_path = parse_exec_path(exec_value)?;

    let version = None; // Version detection is out of scope.

    Some(BrowserInfo {
        kind,
        channel,
        display_name,
        executable_path,
        version,
        unique_id: path.to_str()?.to_string(),
        exec_command: Some(exec_value.to_string()),
    })
}

fn parse_exec_path(exec: &str) -> Option<PathBuf> {
    let parts = shell_words::split(exec).ok()?;
    let first = parts.first()?.clone();
    Some(PathBuf::from(first))
}

fn parse_desktop_file_name(path_str: &str) -> Option<(BrowserKind, BrowserChannel)> {
    let file_name = Path::new(path_str).file_name()?.to_str()?.to_lowercase();
    classify_browser_from_token(&file_name)
}

fn prepare_launch_command(
    info: &BrowserInfo,
    urls: &[String],
) -> Result<(PathBuf, Vec<String>, bool), LaunchError> {
    if let Some(exec_line) = info.exec_command.as_deref() {
        if let Some(parts) = build_command_from_exec(exec_line, info, urls) {
            return Ok(parts);
        }
    }

    let exec = info.launch_path();

    Ok((exec.to_path_buf(), Vec::new(), false))
}

fn build_command_from_exec(
    exec_line: &str,
    info: &BrowserInfo,
    urls: &[String],
) -> Option<(PathBuf, Vec<String>, bool)> {
    let tokens = shell_words::split(exec_line).ok()?;
    let mut iter = tokens.into_iter();
    let program_token = iter.next()?;

    let desktop_path = {
        let path = Path::new(&info.unique_id);
        if info.unique_id.is_empty() {
            None
        } else {
            Some(path)
        }
    };

    let mut args = Vec::new();
    let mut consumed_urls = false;

    for token in iter {
        let (mut expanded, consumed) = expand_exec_token(&token, info, desktop_path, urls);
        if consumed {
            consumed_urls = true;
        }
        args.append(&mut expanded);
    }

    Some((PathBuf::from(program_token), args, consumed_urls))
}

fn expand_exec_token(
    token: &str,
    info: &BrowserInfo,
    desktop_path: Option<&Path>,
    urls: &[String],
) -> (Vec<String>, bool) {
    match token {
        "%u" | "%f" => {
            if let Some(first) = urls.first() {
                (vec![first.clone()], true)
            } else {
                (Vec::new(), false)
            }
        }
        "%U" | "%F" => {
            if urls.is_empty() {
                (Vec::new(), false)
            } else {
                (urls.to_vec(), true)
            }
        }
        "%c" => (vec![info.display_name.clone()], false),
        "%k" => {
            if let Some(path) = desktop_path {
                (vec![path.to_string_lossy().to_string()], false)
            } else {
                (Vec::new(), false)
            }
        }
        "%i" => (Vec::new(), false),
        "%%" => (vec!["%".to_string()], false),
        "%d" | "%D" | "%n" | "%N" | "%m" => (Vec::new(), false),
        _ => {
            let mut consumed = false;
            let mut expanded = token.to_string();

            if expanded.contains("%%") {
                expanded = expanded.replace("%%", "%");
            }

            if expanded.contains("%c") {
                expanded = expanded.replace("%c", &info.display_name);
            }

            if expanded.contains("%k") {
                if let Some(path) = desktop_path {
                    expanded = expanded.replace("%k", &path.to_string_lossy());
                } else {
                    expanded = expanded.replace("%k", "");
                }
            }

            if expanded.contains("%u") || expanded.contains("%f") {
                if let Some(first) = urls.first() {
                    expanded = expanded.replace("%u", first);
                    expanded = expanded.replace("%f", first);
                    consumed = true;
                } else {
                    expanded = expanded.replace("%u", "");
                    expanded = expanded.replace("%f", "");
                }
            }

            if expanded.contains("%U") || expanded.contains("%F") {
                if !urls.is_empty() {
                    if expanded == "%U" || expanded == "%F" {
                        return (urls.to_vec(), true);
                    }

                    expanded = expanded.replace("%U", &urls[0]);
                    expanded = expanded.replace("%F", &urls[0]);
                    consumed = true;
                } else {
                    expanded = expanded.replace("%U", "");
                    expanded = expanded.replace("%F", "");
                }
            }

            (vec![expanded], consumed)
        }
    }
}

fn infer_kind_from_entry(path: &Path, content: &str) -> Option<(BrowserKind, BrowserChannel)> {
    let mut candidates = Vec::new();

    if let Some(name) = get_desktop_entry_value(content, "Name") {
        candidates.push(name.to_string());
    }

    if let Some(exec) = get_desktop_entry_value(content, "Exec") {
        candidates.push(exec.to_string());
        if let Some(exec_path) = parse_exec_path(exec) {
            if let Some(file_name) = exec_path.file_name().and_then(|s| s.to_str()) {
                candidates.push(file_name.to_string());
            }
        }
    }

    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
        candidates.push(stem.to_string());
    }

    for candidate in candidates {
        if let Some(result) = classify_browser_from_token(&candidate.to_ascii_lowercase()) {
            return Some(result);
        }
    }

    None
}

fn classify_browser_from_token(token: &str) -> Option<(BrowserKind, BrowserChannel)> {
    if token.contains("helium") {
        return Some((BrowserKind::Helium, BrowserChannel::Single));
    }

    if token.contains("google-chrome") || token.contains("chrome") {
        let channel = if token.contains("canary") {
            ChromiumChannel::Canary
        } else if token.contains("beta") {
            ChromiumChannel::Beta
        } else if token.contains("dev") {
            ChromiumChannel::Dev
        } else {
            ChromiumChannel::Stable
        };
        return Some((BrowserKind::Chrome, BrowserChannel::Chromium(channel)));
    }

    if token.contains("chromium") {
        return Some((BrowserKind::Chromium, BrowserChannel::Single));
    }

    if token.contains("firefox") {
        let channel = if token.contains("developeredition") || token.contains("developer") {
            FirefoxChannel::Dev
        } else if token.contains("nightly") {
            FirefoxChannel::Nightly
        } else if token.contains("esr") {
            FirefoxChannel::Esr
        } else {
            FirefoxChannel::Stable
        };
        return Some((BrowserKind::Firefox, BrowserChannel::Firefox(channel)));
    }

    if token.contains("edge") || token.contains("microsoft-edge") {
        let channel = if token.contains("beta") {
            ChromiumChannel::Beta
        } else if token.contains("dev") {
            ChromiumChannel::Dev
        } else if token.contains("canary") {
            ChromiumChannel::Canary
        } else {
            ChromiumChannel::Stable
        };
        return Some((BrowserKind::Edge, BrowserChannel::Chromium(channel)));
    }

    if token.contains("brave") {
        let channel = if token.contains("beta") {
            ChromiumChannel::Beta
        } else if token.contains("nightly") {
            ChromiumChannel::Dev
        } else {
            ChromiumChannel::Stable
        };
        return Some((BrowserKind::Brave, BrowserChannel::Chromium(channel)));
    }

    if token.contains("opera") {
        let channel = if token.contains("gx") {
            OperaChannel::Gx
        } else if token.contains("beta") {
            OperaChannel::Beta
        } else {
            OperaChannel::Stable
        };
        return Some((BrowserKind::Opera, BrowserChannel::Opera(channel)));
    }

    if token.contains("vivaldi") {
        return Some((BrowserKind::Vivaldi, BrowserChannel::Single));
    }

    if token.contains("tor") {
        return Some((BrowserKind::TorBrowser, BrowserChannel::Single));
    }

    if token.contains("waterfox") {
        return Some((BrowserKind::Waterfox, BrowserChannel::Single));
    }

    if token.contains("arc") {
        return Some((BrowserKind::Arc, BrowserChannel::Single));
    }

    None
}

fn detect_default_desktop_entry<F: FileSystem>(fs: &F) -> Option<String> {
    for path in candidate_mimeapps_files() {
        if !fs_is_file(fs, &path) {
            continue;
        }

        if let Ok(content) = fs.read_to_string(&path) {
            if let Some(entry) = parse_mimeapps_default(&content) {
                return Some(entry);
            }
        }
    }

    None
}

fn candidate_mimeapps_files() -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Some(config_home) = xdg_config_home() {
        files.push(config_home.join("mimeapps.list"));
        files.push(config_home.join("xdg-desktop-portal/mimeapps.list"));
    }

    if let Some(data_home) = xdg_data_home() {
        files.push(data_home.join("applications/mimeapps.list"));
    }

    files.push(PathBuf::from("/usr/local/share/applications/mimeapps.list"));
    files.push(PathBuf::from("/usr/share/applications/mimeapps.list"));
    files
}

fn xdg_config_home() -> Option<PathBuf> {
    if let Ok(path) = env::var("XDG_CONFIG_HOME") {
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }

    env::var("HOME")
        .ok()
        .map(|home| Path::new(&home).join(".config"))
}

fn xdg_data_home() -> Option<PathBuf> {
    if let Ok(path) = env::var("XDG_DATA_HOME") {
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }

    env::var("HOME")
        .ok()
        .map(|home| Path::new(&home).join(".local/share"))
}

fn parse_mimeapps_default(content: &str) -> Option<String> {
    let mut in_section = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed.eq_ignore_ascii_case("[Default Applications]");
            continue;
        }

        if !in_section {
            continue;
        }

        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };

        if key != "x-scheme-handler/https" && key != "x-scheme-handler/http" {
            continue;
        }

        for entry in value.split(';') {
            let candidate = entry.trim();
            if !candidate.is_empty() {
                return Some(candidate.to_string());
            }
        }
    }

    None
}

fn resolve_desktop_entry_path<F: FileSystem>(fs: &F, desktop_id: &str) -> Option<PathBuf> {
    let mut tried = HashSet::new();

    let with_extension = if desktop_id.ends_with(".desktop") {
        desktop_id.to_string()
    } else {
        format!("{}.desktop", desktop_id)
    };

    // If the ID is already an absolute path, trust it directly.
    let candidate = PathBuf::from(&with_extension);
    if candidate.is_absolute() && fs_is_file(fs, &candidate) {
        return Some(candidate);
    }

    for dir in desktop_file_dirs() {
        let path = dir.join(&with_extension);
        if tried.insert(path.clone()) && fs_is_file(fs, &path) {
            return Some(path);
        }
    }

    None
}
