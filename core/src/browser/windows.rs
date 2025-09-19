use super::{
    BrowserChannel, BrowserInfo, BrowserKind, LaunchCommand, LaunchOutcome, LaunchTarget,
    SystemDefaultBrowser,
};
use std::env;
use std::io;
use std::path::PathBuf;
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

pub fn detect_browsers() -> Vec<BrowserInfo> {
    let mut result = Vec::new();
    for candidate in windows_candidates() {
        if let Some(info) = resolve_candidate(candidate) {
            result.push(info);
        }
    }
    result
}

pub fn system_default_browser() -> Option<SystemDefaultBrowser> {
    None
}

pub fn launch(target: LaunchTarget<'_>, urls: &[String]) -> Result<LaunchOutcome, LaunchError> {
    if urls.is_empty() {
        return Err(LaunchError::NoUrls);
    }

    match target {
        LaunchTarget::Browser(info) => {
            let exec = info
                .launch_path()
                .ok_or_else(|| LaunchError::MissingExecutable(info.display_name.clone()))?;

            let mut command = Command::new(exec);
            command.args(urls);
            command.stdin(Stdio::null());
            command.stdout(Stdio::null());
            command.stderr(Stdio::null());
            debug!(program = %exec.display(), args = ?urls, "Launching browser");
            command.spawn()?;

            let cmd = LaunchCommand {
                program: exec.to_path_buf(),
                args: urls.to_vec(),
                display: format!("{} {}", exec.display(), urls.join(" ")),
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
                let mut command = Command::new("cmd");
                command.arg("/C").arg("start").arg("").arg(url);
                command.stdin(Stdio::null());
                command.stdout(Stdio::null());
                command.stderr(Stdio::null());
                debug!(program = "cmd", url = %url, "Launching system default browser");
                command.spawn()?;
            }

            let cmd = LaunchCommand {
                program: PathBuf::from("cmd"),
                args: urls.to_vec(),
                display: format!("cmd /C start {}", urls.join(" ")),
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

struct WindowsBrowserCandidate {
    kind: BrowserKind,
    channel: BrowserChannel,
    cli_name: &'static str,
    display_name: &'static str,
    relative_paths: &'static [&'static str],
    aliases: &'static [&'static str],
}

fn windows_candidates() -> &'static [WindowsBrowserCandidate] {
    const CANDIDATES: &[WindowsBrowserCandidate] = &[
        WindowsBrowserCandidate {
            kind: BrowserKind::Chrome,
            channel: BrowserChannel::Stable,
            cli_name: "chrome",
            display_name: "Google Chrome",
            relative_paths: &["Google/Chrome/Application/chrome.exe"],
            aliases: &["google-chrome"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Chrome,
            channel: BrowserChannel::Beta,
            cli_name: "chrome-beta",
            display_name: "Google Chrome Beta",
            relative_paths: &["Google/Chrome Beta/Application/chrome.exe"],
            aliases: &["google-chrome-beta"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Chrome,
            channel: BrowserChannel::Dev,
            cli_name: "chrome-dev",
            display_name: "Google Chrome Dev",
            relative_paths: &["Google/Chrome Dev/Application/chrome.exe"],
            aliases: &["google-chrome-dev"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Chrome,
            channel: BrowserChannel::Canary,
            cli_name: "chrome-canary",
            display_name: "Google Chrome Canary",
            relative_paths: &["Google/Chrome SxS/Application/chrome.exe"],
            aliases: &["google-chrome-canary"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Firefox,
            channel: BrowserChannel::Stable,
            cli_name: "firefox",
            display_name: "Mozilla Firefox",
            relative_paths: &["Mozilla Firefox/firefox.exe"],
            aliases: &["mozilla-firefox"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Firefox,
            channel: BrowserChannel::Dev,
            cli_name: "firefox-developer",
            display_name: "Firefox Developer Edition",
            relative_paths: &["Firefox Developer Edition/firefox.exe"],
            aliases: &["firefox-dev"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Firefox,
            channel: BrowserChannel::Nightly,
            cli_name: "firefox-nightly",
            display_name: "Firefox Nightly",
            relative_paths: &["Firefox Nightly/firefox.exe"],
            aliases: &["firefox-nightly"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Edge,
            channel: BrowserChannel::Stable,
            cli_name: "edge",
            display_name: "Microsoft Edge",
            relative_paths: &["Microsoft/Edge/Application/msedge.exe"],
            aliases: &["microsoft-edge"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Edge,
            channel: BrowserChannel::Beta,
            cli_name: "edge-beta",
            display_name: "Microsoft Edge Beta",
            relative_paths: &["Microsoft/Edge Beta/Application/msedge.exe"],
            aliases: &["microsoft-edge-beta"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Edge,
            channel: BrowserChannel::Dev,
            cli_name: "edge-dev",
            display_name: "Microsoft Edge Dev",
            relative_paths: &["Microsoft/Edge Dev/Application/msedge.exe"],
            aliases: &["microsoft-edge-dev"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Edge,
            channel: BrowserChannel::Canary,
            cli_name: "edge-canary",
            display_name: "Microsoft Edge Canary",
            relative_paths: &["Microsoft/Edge SxS/Application/msedge.exe"],
            aliases: &["microsoft-edge-canary"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Brave,
            channel: BrowserChannel::Stable,
            cli_name: "brave",
            display_name: "Brave Browser",
            relative_paths: &["BraveSoftware/Brave-Browser/Application/brave.exe"],
            aliases: &["brave-browser"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Brave,
            channel: BrowserChannel::Beta,
            cli_name: "brave-beta",
            display_name: "Brave Browser Beta",
            relative_paths: &["BraveSoftware/Brave-Browser-Beta/Application/brave.exe"],
            aliases: &["brave-browser-beta"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Brave,
            channel: BrowserChannel::Dev,
            cli_name: "brave-dev",
            display_name: "Brave Browser Dev",
            relative_paths: &["BraveSoftware/Brave-Browser-Dev/Application/brave.exe"],
            aliases: &["brave-browser-dev"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Brave,
            channel: BrowserChannel::Nightly,
            cli_name: "brave-nightly",
            display_name: "Brave Browser Nightly",
            relative_paths: &["BraveSoftware/Brave-Browser-Nightly/Application/brave.exe"],
            aliases: &["brave-browser-nightly"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Vivaldi,
            channel: BrowserChannel::Stable,
            cli_name: "vivaldi",
            display_name: "Vivaldi",
            relative_paths: &["Vivaldi/Application/vivaldi.exe"],
            aliases: &["vivaldi-browser"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Opera,
            channel: BrowserChannel::Stable,
            cli_name: "opera",
            display_name: "Opera",
            relative_paths: &["Opera/opera.exe"],
            aliases: &["opera-browser"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::TorBrowser,
            channel: BrowserChannel::Stable,
            cli_name: "tor",
            display_name: "Tor Browser",
            relative_paths: &["Tor Browser/Browser/firefox.exe"],
            aliases: &["tor-browser"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Chromium,
            channel: BrowserChannel::Stable,
            cli_name: "chromium",
            display_name: "Chromium",
            relative_paths: &["Chromium/Application/chromium.exe"],
            aliases: &["chromium-browser"],
        },
        WindowsBrowserCandidate {
            kind: BrowserKind::Waterfox,
            channel: BrowserChannel::Stable,
            cli_name: "waterfox",
            display_name: "Waterfox",
            relative_paths: &["Waterfox/waterfox.exe"],
            aliases: &["waterfox-browser"],
        },
    ];

    CANDIDATES
}

fn resolve_candidate(candidate: &WindowsBrowserCandidate) -> Option<BrowserInfo> {
    for base in windows_base_dirs() {
        for relative in candidate.relative_paths {
            let path = base.join(relative);
            if path.exists() {
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
                    bundle_path: path.parent().map(|p| p.to_path_buf()),
                    executable: Some(path.clone()),
                    version: None,
                    source: Some("windows".to_string()),
                });
            }
        }
    }

    None
}

fn windows_base_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(path) = env::var_os("PROGRAMFILES") {
        dirs.push(PathBuf::from(path));
    }
    if let Some(path) = env::var_os("PROGRAMFILES(X86)") {
        dirs.push(PathBuf::from(path));
    }
    if let Some(path) = env::var_os("LOCALAPPDATA") {
        dirs.push(PathBuf::from(path));
    }
    dirs
}
