use super::{
    BrowserChannel, BrowserInfo, BrowserKind, LaunchCommand, LaunchOutcome, LaunchTarget,
    SystemDefaultBrowser,
};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{fs, io};
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
    let candidates = mac_candidates();

    for candidate in candidates {
        if let Some(info) = resolve_candidate(candidate) {
            result.push(info);
        }
    }

    result
}

pub fn system_default_browser() -> Option<SystemDefaultBrowser> {
    if let Some(bundle_id) = default_handler_for_scheme("https") {
        if let Some(candidate) = mac_candidates()
            .iter()
            .find(|candidate| candidate.bundle_ids.contains(&bundle_id.as_str()))
        {
            let mut path = None;
            for base in mac_base_paths() {
                if let Some(bundle_path) = locate_bundle_in_base(base, candidate) {
                    path = Some(bundle_path);
                    break;
                }
            }

            return Some(SystemDefaultBrowser {
                identifier: bundle_id,
                display_name: candidate.display_name.to_string(),
                kind: Some(candidate.kind),
                channel: Some(candidate.channel),
                path,
            });
        }

        return Some(SystemDefaultBrowser {
            identifier: bundle_id,
            display_name: "System default".to_string(),
            kind: None,
            channel: None,
            path: None,
        });
    }

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
            let mut command = Command::new("open");
            command.args(urls);
            command.stdin(Stdio::null());
            command.stdout(Stdio::null());
            command.stderr(Stdio::null());
            debug!(program = "open", args = ?urls, "Launching system default browser");
            command.spawn()?;

            let cmd = LaunchCommand {
                program: PathBuf::from("open"),
                args: urls.to_vec(),
                display: format!("open {}", urls.join(" ")),
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

struct MacBrowserCandidate {
    kind: BrowserKind,
    channel: BrowserChannel,
    cli_name: &'static str,
    display_name: &'static str,
    bundle_names: &'static [&'static str],
    executable_name: &'static str,
    aliases: &'static [&'static str],
    bundle_ids: &'static [&'static str],
}

fn mac_candidates() -> &'static [MacBrowserCandidate] {
    const CANDIDATES: &[MacBrowserCandidate] = &[
        MacBrowserCandidate {
            kind: BrowserKind::Chrome,
            channel: BrowserChannel::Stable,
            cli_name: "chrome",
            display_name: "Google Chrome",
            bundle_names: &["Google Chrome.app"],
            executable_name: "Google Chrome",
            aliases: &["google-chrome", "chrome-stable"],
            bundle_ids: &["com.google.Chrome"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Chrome,
            channel: BrowserChannel::Beta,
            cli_name: "chrome-beta",
            display_name: "Google Chrome Beta",
            bundle_names: &["Google Chrome Beta.app"],
            executable_name: "Google Chrome Beta",
            aliases: &["google-chrome-beta"],
            bundle_ids: &["com.google.Chrome.beta"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Chrome,
            channel: BrowserChannel::Dev,
            cli_name: "chrome-dev",
            display_name: "Google Chrome Dev",
            bundle_names: &["Google Chrome Dev.app"],
            executable_name: "Google Chrome Dev",
            aliases: &["google-chrome-dev"],
            bundle_ids: &["com.google.Chrome.dev"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Chrome,
            channel: BrowserChannel::Canary,
            cli_name: "chrome-canary",
            display_name: "Google Chrome Canary",
            bundle_names: &["Google Chrome Canary.app"],
            executable_name: "Google Chrome Canary",
            aliases: &["google-chrome-canary", "chrome-can"],
            bundle_ids: &["com.google.Chrome.canary"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Firefox,
            channel: BrowserChannel::Stable,
            cli_name: "firefox",
            display_name: "Firefox",
            bundle_names: &["Firefox.app"],
            executable_name: "firefox",
            aliases: &["mozilla-firefox", "firefox-stable"],
            bundle_ids: &["org.mozilla.firefox"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Firefox,
            channel: BrowserChannel::Dev,
            cli_name: "firefox-developer",
            display_name: "Firefox Developer Edition",
            bundle_names: &["Firefox Developer Edition.app"],
            executable_name: "firefox",
            aliases: &["firefox-dev", "firefox-developer-edition"],
            bundle_ids: &["org.mozilla.firefoxdeveloperedition"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Firefox,
            channel: BrowserChannel::Nightly,
            cli_name: "firefox-nightly",
            display_name: "Firefox Nightly",
            bundle_names: &["Firefox Nightly.app"],
            executable_name: "firefox",
            aliases: &["firefox-night"],
            bundle_ids: &["org.mozilla.nightly"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Safari,
            channel: BrowserChannel::Stable,
            cli_name: "safari",
            display_name: "Safari",
            bundle_names: &["Safari.app"],
            executable_name: "Safari",
            aliases: &["apple-safari"],
            bundle_ids: &["com.apple.Safari"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Edge,
            channel: BrowserChannel::Stable,
            cli_name: "edge",
            display_name: "Microsoft Edge",
            bundle_names: &["Microsoft Edge.app"],
            executable_name: "Microsoft Edge",
            aliases: &["microsoft-edge"],
            bundle_ids: &["com.microsoft.edgemac"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Edge,
            channel: BrowserChannel::Beta,
            cli_name: "edge-beta",
            display_name: "Microsoft Edge Beta",
            bundle_names: &["Microsoft Edge Beta.app"],
            executable_name: "Microsoft Edge Beta",
            aliases: &["microsoft-edge-beta"],
            bundle_ids: &["com.microsoft.edgemac.beta"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Edge,
            channel: BrowserChannel::Dev,
            cli_name: "edge-dev",
            display_name: "Microsoft Edge Dev",
            bundle_names: &["Microsoft Edge Dev.app"],
            executable_name: "Microsoft Edge Dev",
            aliases: &["microsoft-edge-dev"],
            bundle_ids: &["com.microsoft.edgemac.dev"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Edge,
            channel: BrowserChannel::Canary,
            cli_name: "edge-canary",
            display_name: "Microsoft Edge Canary",
            bundle_names: &["Microsoft Edge Canary.app"],
            executable_name: "Microsoft Edge Canary",
            aliases: &["microsoft-edge-canary"],
            bundle_ids: &["com.microsoft.edgemac.canary"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Brave,
            channel: BrowserChannel::Stable,
            cli_name: "brave",
            display_name: "Brave Browser",
            bundle_names: &["Brave Browser.app"],
            executable_name: "Brave Browser",
            aliases: &["brave-browser"],
            bundle_ids: &["com.brave.Browser"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Brave,
            channel: BrowserChannel::Beta,
            cli_name: "brave-beta",
            display_name: "Brave Browser Beta",
            bundle_names: &["Brave Browser Beta.app"],
            executable_name: "Brave Browser Beta",
            aliases: &["brave-browser-beta"],
            bundle_ids: &["com.brave.Browser.beta"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Brave,
            channel: BrowserChannel::Dev,
            cli_name: "brave-dev",
            display_name: "Brave Browser Dev",
            bundle_names: &["Brave Browser Dev.app"],
            executable_name: "Brave Browser Dev",
            aliases: &["brave-browser-dev"],
            bundle_ids: &["com.brave.Browser.dev"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Brave,
            channel: BrowserChannel::Nightly,
            cli_name: "brave-nightly",
            display_name: "Brave Browser Nightly",
            bundle_names: &["Brave Browser Nightly.app"],
            executable_name: "Brave Browser Nightly",
            aliases: &["brave-browser-nightly"],
            bundle_ids: &["com.brave.Browser.nightly"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Arc,
            channel: BrowserChannel::Stable,
            cli_name: "arc",
            display_name: "Arc",
            bundle_names: &["Arc.app"],
            executable_name: "Arc",
            aliases: &["the-browser-arc"],
            bundle_ids: &["company.thebrowser.Browser"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Helium,
            channel: BrowserChannel::Stable,
            cli_name: "helium",
            display_name: "Helium",
            bundle_names: &["Helium.app"],
            executable_name: "Helium",
            aliases: &["helium-browser"],
            bundle_ids: &["net.imput.helium"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Vivaldi,
            channel: BrowserChannel::Stable,
            cli_name: "vivaldi",
            display_name: "Vivaldi",
            bundle_names: &["Vivaldi.app"],
            executable_name: "Vivaldi",
            aliases: &["vivaldi-browser"],
            bundle_ids: &["com.vivaldi.Vivaldi"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Opera,
            channel: BrowserChannel::Stable,
            cli_name: "opera",
            display_name: "Opera",
            bundle_names: &["Opera.app"],
            executable_name: "Opera",
            aliases: &["opera-browser"],
            bundle_ids: &["com.operasoftware.Opera"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::TorBrowser,
            channel: BrowserChannel::Stable,
            cli_name: "tor",
            display_name: "Tor Browser",
            bundle_names: &["Tor Browser.app"],
            executable_name: "Tor Browser",
            aliases: &["tor-browser"],
            bundle_ids: &["org.torproject.torbrowser"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Chromium,
            channel: BrowserChannel::Stable,
            cli_name: "chromium",
            display_name: "Chromium",
            bundle_names: &["Chromium.app"],
            executable_name: "Chromium",
            aliases: &[],
            bundle_ids: &["org.chromium.Chromium"],
        },
        MacBrowserCandidate {
            kind: BrowserKind::Waterfox,
            channel: BrowserChannel::Stable,
            cli_name: "waterfox",
            display_name: "Waterfox",
            bundle_names: &["Waterfox.app", "Waterfox Current.app"],
            executable_name: "Waterfox",
            aliases: &["waterfox-current"],
            bundle_ids: &["net.waterfox.waterfox"],
        },
    ];

    CANDIDATES
}

fn resolve_candidate(candidate: &MacBrowserCandidate) -> Option<BrowserInfo> {
    for base in mac_base_paths() {
        if let Some(bundle_path) = locate_bundle_in_base(base, candidate) {
            let exec_path = bundle_path
                .join("Contents")
                .join("MacOS")
                .join(candidate.executable_name);

            if !exec_path.exists() {
                continue;
            }

            if !is_executable(&exec_path) {
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
                bundle_path: Some(bundle_path),
                executable: Some(exec_path),
                version: None,
                source: Some("macos".to_string()),
            });
        }
    }

    None
}

fn mac_base_paths() -> Vec<PathBuf> {
    let mut bases = vec![PathBuf::from("/Applications")];
    if let Ok(home) = env::var("HOME") {
        bases.push(Path::new(&home).join("Applications"));
    }
    bases
}

fn locate_bundle_in_base(base: PathBuf, candidate: &MacBrowserCandidate) -> Option<PathBuf> {
    for bundle in candidate.bundle_names {
        let path = base.join(bundle);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

#[cfg(target_family = "unix")]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = fs::metadata(path) {
        let permissions = metadata.permissions();
        permissions.mode() & 0o111 != 0
    } else {
        false
    }
}

#[cfg(not(target_family = "unix"))]
fn is_executable(path: &Path) -> bool {
    path.exists()
}

fn default_handler_for_scheme(scheme: &str) -> Option<String> {
    let output = Command::new("/usr/bin/defaults")
        .arg("read")
        .arg("com.apple.LaunchServices/com.apple.launchservices.secure")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    parse_defaults_for_scheme(&String::from_utf8_lossy(&output.stdout), scheme)
}

fn parse_defaults_for_scheme(data: &str, scheme: &str) -> Option<String> {
    let mut current_scheme: Option<String> = None;
    let mut current_handler: Option<String> = None;

    for line in data.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("LSHandlerURLScheme") {
            current_scheme = parse_defaults_value(trimmed);
        } else if trimmed.starts_with("LSHandlerRoleAll") {
            current_handler = parse_defaults_value(trimmed);
        } else if trimmed.starts_with("LSHandlerRoleViewer") {
            current_handler = parse_defaults_value(trimmed);
        }

        if trimmed == "};" || trimmed == "}" {
            if let (Some(s), Some(handler)) = (&current_scheme, &current_handler) {
                if s == scheme {
                    return Some(handler.clone());
                }
            }
            current_scheme = None;
            current_handler = None;
        }
    }

    None
}

fn parse_defaults_value(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.split('=').collect();
    if parts.len() != 2 {
        return None;
    }
    let value = parts[1].trim().trim_end_matches(';').trim();
    Some(value.trim_matches('"').to_string())
}
