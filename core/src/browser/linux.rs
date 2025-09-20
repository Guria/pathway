use super::{
    BrowserChannel, BrowserInfo, BrowserKind, LaunchCommand, LaunchOutcome, LaunchTarget,
    SystemDefaultBrowser,
};
use crate::filesystem::{FileSystem, RealFileSystem};
use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("Browser executable missing: {0}")]
    MissingExecutable(String),
    #[error("No URLs provided to launch")]
    NoUrls,
    #[error("Failed to launch browser: {source}")]
    Spawn {
        #[from]
        source: io::Error,
    },
}

pub fn detect_browsers<F: FileSystem>(fs: &F) -> Vec<BrowserInfo> {
    let mut result = Vec::new();
    let candidates = linux_candidates();

    for candidate in candidates {
        if let Some(info) = resolve_candidate(candidate, fs) {
            result.push(info);
        }
    }

    result
}

/// Determine the system default browser on Linux.
///
/// Queries the desktop environment's default web browser entry (via xdg-settings) and attempts to
/// map that desktop entry to a known browser candidate. If a matching candidate is found this
/// returns a `SystemDefaultBrowser` populated with the desktop entry identifier, the candidate's
/// display name, kind and channel. If the candidate's executable can be resolved on the system,
/// `path` will contain the executable path; otherwise `path` is `None`.
///
/// Returns `None` if the default desktop entry cannot be determined.
///
/// # Examples
///
/// ```no_run
/// use pathway::detect_inventory;
///
/// let inventory = detect_inventory();
/// let sys = &inventory.system_default;
/// println!("Default browser: {} ({})", sys.display_name, sys.identifier);
/// ```
pub fn system_default_browser() -> Option<SystemDefaultBrowser> {
    if let Some(entry) = query_default_desktop_entry() {
        if let Some(candidate) = linux_candidates()
            .iter()
            .find(|candidate| candidate.desktop_entries.contains(&entry.as_str()))
        {
            if let Some(info) = resolve_candidate(candidate, &RealFileSystem) {
                return Some(SystemDefaultBrowser {
                    identifier: entry.clone(),
                    display_name: candidate.display_name.to_string(),
                    kind: Some(candidate.kind),
                    channel: Some(candidate.channel),
                    path: info.executable.clone(),
                });
            }

            return Some(SystemDefaultBrowser {
                identifier: entry,
                display_name: candidate.display_name.to_string(),
                kind: Some(candidate.kind),
                channel: Some(candidate.channel),
                path: None,
            });
        }

        return Some(SystemDefaultBrowser {
            identifier: entry,
            display_name: "System default".to_string(),
            kind: None,
            channel: None,
            path: None,
        });
    }

    None
}

/// Launches the given URLs using the specified launch target.
///
/// This is a convenience wrapper around `launch_with_profile` that does not
/// supply profile or window options.
///
/// # Examples
///
/// ```no_run
/// use pathway::browser::{launch, LaunchTarget};
///
/// let urls = vec!["https://example.com".to_string()];
/// // Launch using the system default browser
/// let _ = launch(LaunchTarget::SystemDefault, &urls);
/// ```
pub fn launch(target: LaunchTarget<'_>, urls: &[String]) -> Result<LaunchOutcome, LaunchError> {
    launch_with_profile(target, urls, None, None)
}

/// Launches one or more URLs using either a specific browser or the system default, optionally applying profile and window options.
///
/// If `target` is `LaunchTarget::Browser`, this will resolve the browser executable and spawn it with the provided URLs. If both `profile_opts` and `window_opts` are supplied, profile-specific CLI arguments are generated and prepended to the browser command.
/// If `target` is `LaunchTarget::SystemDefault`, each URL is opened via `xdg-open`.
///
/// Returns `Err(LaunchError::NoUrls)` when `urls` is empty. Returns `Err(LaunchError::MissingExecutable(...))` when the chosen browser has no resolvable executable. Spawn failures are returned as `LaunchError::Spawn { source }`.
///
/// # Examples
///
/// ```
/// # use std::path::PathBuf;
/// # use pathway::browser::LaunchTarget;
/// # use pathway::browser::launch_with_profile;
/// // Open two URLs with the system default browser
/// let urls = vec!["https://example.com".to_string(), "https://rust-lang.org".to_string()];
/// let outcome = launch_with_profile(LaunchTarget::SystemDefault, &urls, None, None);
/// assert!(outcome.is_ok());
/// ```
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
            let exec = info
                .launch_path()
                .ok_or_else(|| LaunchError::MissingExecutable(info.display_name.clone()))?;

            let mut command = Command::new(exec);

            let has_profile_args =
                if let (Some(profile_opts), Some(window_opts)) = (profile_opts, window_opts) {
                    let profile_args = crate::profile::ProfileManager::generate_profile_args(
                        info,
                        profile_opts,
                        window_opts,
                    );
                    command.args(&profile_args);
                    !profile_args.is_empty()
                } else {
                    false
                };

            command.args(urls);
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
            debug!(program = %exec.display(), args = ?all_args, "{}", log_message);
            command.spawn()?;

            let cmd = LaunchCommand {
                program: exec.to_path_buf(),
                args: all_args.clone(),
                display: format!("{} {}", exec.display(), all_args.join(" ")),
                is_system_default: false,
            };

            Ok(LaunchOutcome {
                browser: Some(info.clone()),
                system_default: None,
                command: cmd,
            })
        }
        LaunchTarget::SystemDefault => {
            for url in urls {
                let mut command = Command::new("xdg-open");

                if let Some(window_opts) = window_opts {
                    if window_opts.new_window {
                        // xdg-open doesn't support a new window flag, so we log this limitation
                        debug!("new-window option requested but xdg-open has no new-window flag - option ignored");
                    }
                }

                command.arg(url);
                command.stdin(Stdio::null());
                command.stdout(Stdio::null());
                command.stderr(Stdio::null());
                debug!(program = "xdg-open", url = %url, "Launching system default browser");
                command.spawn()?;
            }

            let cmd = LaunchCommand {
                program: PathBuf::from("xdg-open"),
                args: urls.to_vec(),
                display: urls
                    .iter()
                    .map(|u| format!("xdg-open {}", u))
                    .collect::<Vec<_>>()
                    .join(" && "),
                is_system_default: true,
            };

            Ok(LaunchOutcome {
                browser: None,
                system_default: system_default_browser(),
                command: cmd,
            })
        }
    }
}

struct LinuxBrowserCandidate {
    kind: BrowserKind,
    channel: BrowserChannel,
    cli_name: &'static str,
    display_name: &'static str,
    binary_names: &'static [&'static str],
    aliases: &'static [&'static str],
    desktop_entries: &'static [&'static str],
}

fn linux_candidates() -> &'static [LinuxBrowserCandidate] {
    const CANDIDATES: &[LinuxBrowserCandidate] = &[
        LinuxBrowserCandidate {
            kind: BrowserKind::Chrome,
            channel: BrowserChannel::Stable,
            cli_name: "chrome",
            display_name: "Google Chrome",
            binary_names: &["google-chrome", "chrome", "google-chrome-stable"],
            aliases: &["google-chrome", "chrome-stable"],
            desktop_entries: &[
                "google-chrome.desktop",
                "chrome.desktop",
                "google-chrome-stable.desktop",
            ],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Chrome,
            channel: BrowserChannel::Beta,
            cli_name: "chrome-beta",
            display_name: "Google Chrome Beta",
            binary_names: &["google-chrome-beta"],
            aliases: &["chrome-beta"],
            desktop_entries: &["google-chrome-beta.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Chrome,
            channel: BrowserChannel::Dev,
            cli_name: "chrome-dev",
            display_name: "Google Chrome Dev",
            binary_names: &["google-chrome-dev"],
            aliases: &["chrome-dev"],
            desktop_entries: &["google-chrome-dev.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Chrome,
            channel: BrowserChannel::Canary,
            cli_name: "chrome-canary",
            display_name: "Google Chrome Canary",
            binary_names: &["google-chrome-canary"],
            aliases: &["chrome-canary"],
            desktop_entries: &["google-chrome-canary.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Firefox,
            channel: BrowserChannel::Stable,
            cli_name: "firefox",
            display_name: "Firefox",
            binary_names: &["firefox"],
            aliases: &["mozilla-firefox"],
            desktop_entries: &["firefox.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Firefox,
            channel: BrowserChannel::Dev,
            cli_name: "firefox-developer",
            display_name: "Firefox Developer Edition",
            binary_names: &["firefox-developer-edition", "firefox-developer"],
            aliases: &["firefox-dev"],
            desktop_entries: &[
                "firefoxdeveloperedition.desktop",
                "firefox-developer-edition.desktop",
            ],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Firefox,
            channel: BrowserChannel::Nightly,
            cli_name: "firefox-nightly",
            display_name: "Firefox Nightly",
            binary_names: &["firefox-nightly"],
            aliases: &["firefox-night"],
            desktop_entries: &["firefox-nightly.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Edge,
            channel: BrowserChannel::Stable,
            cli_name: "edge",
            display_name: "Microsoft Edge",
            binary_names: &["microsoft-edge", "microsoft-edge-stable"],
            aliases: &["edge-stable"],
            desktop_entries: &["microsoft-edge.desktop", "microsoft-edge-stable.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Edge,
            channel: BrowserChannel::Beta,
            cli_name: "edge-beta",
            display_name: "Microsoft Edge Beta",
            binary_names: &["microsoft-edge-beta"],
            aliases: &["edge-beta"],
            desktop_entries: &["microsoft-edge-beta.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Edge,
            channel: BrowserChannel::Dev,
            cli_name: "edge-dev",
            display_name: "Microsoft Edge Dev",
            binary_names: &["microsoft-edge-dev"],
            aliases: &["edge-dev"],
            desktop_entries: &["microsoft-edge-dev.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Edge,
            channel: BrowserChannel::Canary,
            cli_name: "edge-canary",
            display_name: "Microsoft Edge Canary",
            binary_names: &["microsoft-edge-canary"],
            aliases: &["edge-canary"],
            desktop_entries: &["microsoft-edge-canary.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Brave,
            channel: BrowserChannel::Stable,
            cli_name: "brave",
            display_name: "Brave Browser",
            binary_names: &["brave", "brave-browser"],
            aliases: &["brave-browser"],
            desktop_entries: &["brave-browser.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Brave,
            channel: BrowserChannel::Beta,
            cli_name: "brave-beta",
            display_name: "Brave Browser Beta",
            binary_names: &["brave-browser-beta"],
            aliases: &["brave-beta"],
            desktop_entries: &["brave-browser-beta.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Brave,
            channel: BrowserChannel::Dev,
            cli_name: "brave-dev",
            display_name: "Brave Browser Dev",
            binary_names: &["brave-browser-dev"],
            aliases: &["brave-dev"],
            desktop_entries: &["brave-browser-dev.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Brave,
            channel: BrowserChannel::Nightly,
            cli_name: "brave-nightly",
            display_name: "Brave Browser Nightly",
            binary_names: &["brave-browser-nightly"],
            aliases: &["brave-nightly"],
            desktop_entries: &["brave-browser-nightly.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Arc,
            channel: BrowserChannel::Stable,
            cli_name: "arc",
            display_name: "Arc",
            binary_names: &["arc"],
            aliases: &[],
            desktop_entries: &["company.thebrowser.Arc.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Vivaldi,
            channel: BrowserChannel::Stable,
            cli_name: "vivaldi",
            display_name: "Vivaldi",
            binary_names: &["vivaldi", "vivaldi-stable"],
            aliases: &["vivaldi-browser"],
            desktop_entries: &["vivaldi.desktop", "vivaldi-stable.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Opera,
            channel: BrowserChannel::Stable,
            cli_name: "opera",
            display_name: "Opera",
            binary_names: &["opera"],
            aliases: &["opera-browser"],
            desktop_entries: &["opera.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::TorBrowser,
            channel: BrowserChannel::Stable,
            cli_name: "tor",
            display_name: "Tor Browser",
            binary_names: &["tor-browser", "tor-browser-en"],
            aliases: &["tor-browser"],
            desktop_entries: &["torbrowser.desktop", "tor-browser.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Chromium,
            channel: BrowserChannel::Stable,
            cli_name: "chromium",
            display_name: "Chromium",
            binary_names: &["chromium", "chromium-browser"],
            aliases: &["chromium-browser"],
            desktop_entries: &["chromium.desktop", "chromium-browser.desktop"],
        },
        LinuxBrowserCandidate {
            kind: BrowserKind::Waterfox,
            channel: BrowserChannel::Stable,
            cli_name: "waterfox",
            display_name: "Waterfox",
            binary_names: &["waterfox"],
            aliases: &["waterfox-browser"],
            desktop_entries: &["waterfox.desktop", "org.waterfoxproject.waterfox.desktop"],
        },
    ];

    CANDIDATES
}

fn resolve_candidate<F: FileSystem>(
    candidate: &LinuxBrowserCandidate,
    fs: &F,
) -> Option<BrowserInfo> {
    let search_dirs = linux_search_directories(fs);

    for binary in candidate.binary_names {
        let potential = locate_executable(binary, &search_dirs, fs);
        if let Some(exec_path) = potential {
            if !is_executable(&exec_path, fs) {
                continue;
            }

            return Some(BrowserInfo {
                id: candidate.cli_name.to_string(),
                cli_name: candidate.cli_name.to_string(),
                display_name: candidate.display_name.to_string(),
                kind: candidate.kind,
                channel: candidate.channel,
                aliases: candidate
                    .aliases
                    .iter()
                    .map(|alias| alias.to_string())
                    .collect(),
                bundle_path: exec_path.parent().map(|p| p.to_path_buf()),
                executable: Some(exec_path.clone()),
                bundle_id: None,
                version: None,
                source: Some("linux".to_string()),
            });
        }
    }

    None
}

fn linux_search_directories<F: FileSystem>(fs: &F) -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/snap/bin"),
        PathBuf::from("/opt"),
    ];

    if let Ok(home) = env::var("HOME") {
        dirs.push(Path::new(&home).join(".local/bin"));
        dirs.push(Path::new(&home).join("bin"));
    }

    dirs.extend(flatpak_bin_dirs(fs));

    dirs
}

fn flatpak_bin_dirs<F: FileSystem>(fs: &F) -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/var/lib/flatpak/exports/bin")];
    if let Ok(home) = env::var("HOME") {
        dirs.push(Path::new(&home).join(".local/share/flatpak/exports/bin"));
    }

    // Only add paths that actually exist
    dirs.into_iter().filter(|path| fs.exists(path)).collect()
}

fn locate_executable<F: FileSystem>(binary: &str, dirs: &[PathBuf], fs: &F) -> Option<PathBuf> {
    let candidate_path = Path::new(binary);
    if candidate_path.is_absolute() && fs.exists(candidate_path) {
        return Some(candidate_path.to_path_buf());
    }

    for dir in dirs {
        let path = dir.join(binary);
        if fs.exists(&path) {
            return Some(path);
        }
    }

    None
}

#[cfg(target_family = "unix")]
fn is_executable<F: FileSystem>(path: &Path, fs: &F) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = fs.metadata(path) {
        let permissions = metadata.permissions();
        permissions.mode() & 0o111 != 0
    } else {
        false
    }
}

#[cfg(not(target_family = "unix"))]
fn is_executable<F: FileSystem>(path: &Path, fs: &F) -> bool {
    fs.exists(path)
}

fn query_default_desktop_entry() -> Option<String> {
    let output = Command::new("xdg-settings")
        .arg("get")
        .arg("default-web-browser")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::MockFileSystem;
    use std::path::PathBuf;

    #[test]
    fn test_browser_detection_with_mock_fs() {
        let mut mock_fs = MockFileSystem::new();

        // Mock that only Chrome exists and is executable
        mock_fs
            .expect_exists()
            .returning(|path| path == Path::new("/usr/bin/google-chrome"));

        // Mock executable permissions check for Chrome
        mock_fs
            .expect_metadata()
            .with(mockall::predicate::eq(Path::new("/usr/bin/google-chrome")))
            .returning(|_| {
                std::fs::metadata("/")
                    .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "mock"))
            });

        let browsers = detect_browsers(&mock_fs);

        // With our mock, we should detect Chrome
        assert!(browsers.iter().any(|b| b.kind == BrowserKind::Chrome));
    }

    #[test]
    fn test_browser_detection_no_browsers() {
        let mut mock_fs = MockFileSystem::new();

        // Mock that no browser executables exist
        mock_fs.expect_exists().returning(|_| false);

        let browsers = detect_browsers(&mock_fs);

        // Should find no browsers
        assert!(browsers.is_empty());
    }

    #[test]
    fn test_locate_executable_mock() {
        let mut mock_fs = MockFileSystem::new();

        mock_fs
            .expect_exists()
            .with(mockall::predicate::eq(Path::new("/usr/bin/chrome")))
            .return_const(true);

        let dirs = vec![PathBuf::from("/usr/bin")];
        let result = locate_executable("chrome", &dirs, &mock_fs);

        assert_eq!(result, Some(PathBuf::from("/usr/bin/chrome")));
    }
}
