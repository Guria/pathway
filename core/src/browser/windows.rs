use super::{BrowserInfo, BrowserKind};
use crate::browser::channels::{BrowserChannel, ChromiumChannel, FirefoxChannel, OperaChannel};
use crate::filesystem::FileSystem;
use std::path::PathBuf;
use winreg::enums::*;
use winreg::RegKey;

use super::{LaunchCommand, LaunchOutcome, LaunchTarget, SystemDefaultBrowser};
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
            // Use cmd /c start to open with system default browser
            let mut command = Command::new("cmd");
            command.arg("/c").arg("start").arg("");
            command.args(urls);
            command.stdin(Stdio::null());
            command.stdout(Stdio::null());
            command.stderr(Stdio::null());

            let all_args: Vec<String> = command
                .get_args()
                .map(|s| s.to_string_lossy().to_string())
                .collect();
            debug!(program = "cmd", args = ?all_args, "Launching system default browser");
            command.spawn()?;

            let cmd = LaunchCommand {
                program: PathBuf::from("cmd"),
                args: all_args.clone(),
                display: format!("cmd {}", all_args.join(" ")),
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
pub fn system_default_browser_with_fs<F: FileSystem>(_fs: &F) -> Option<SystemDefaultBrowser> {
    let prog_id = default_prog_id()?;

    if let Some(info) = browser_info_for_prog_id(&prog_id) {
        let path = info.launch_path().map(|p| p.to_path_buf());
        return Some(SystemDefaultBrowser {
            identifier: prog_id,
            display_name: info.display_name,
            kind: Some(info.kind),
            path,
        });
    }

    fallback_system_default(&prog_id)
}
// End stubs

pub fn detect_browsers<F: FileSystem>(_fs: &F) -> Vec<BrowserInfo> {
    let mut browsers = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let search_path = "SOFTWARE\\Clients\\StartMenuInternet";

    // Search both HKLM and HKCU
    for key in [&hklm, &hkcu] {
        if let Ok(internet_clients) = key.open_subkey(search_path) {
            for client_name in internet_clients.enum_keys().filter_map(Result::ok) {
                if let Some(browser_info) = create_browser_info(key, search_path, &client_name) {
                    browsers.push(browser_info);
                }
            }
        }
    }

    // TODO: Deduplicate browsers
    browsers
}

fn create_browser_info(
    base_key: &RegKey,
    search_path: &str,
    client_name: &str,
) -> Option<BrowserInfo> {
    let reg_path = format!("{}\\{}", search_path, client_name);
    let client_key = base_key.open_subkey(&reg_path).ok()?;

    let display_name: String = client_key.get_value("").ok()?;
    let (kind, channel) = parse_client_name(&display_name, client_name)?;

    let command_path = client_key
        .open_subkey("shell\\open\\command")
        .ok()?
        .get_value::<String, _>("")
        .ok()?;

    let executable_path = parse_command_path(&command_path)?;

    // Version detection is complex, requires reading file properties.
    let version = None;

    Some(BrowserInfo {
        kind,
        channel,
        display_name,
        executable_path,
        version,
        unique_id: reg_path,
        exec_command: Some(command_path),
    })
}

fn parse_command_path(command: &str) -> Option<PathBuf> {
    // The command might be quoted and contain arguments.
    let path_str = if command.starts_with('"') {
        command.split('"').nth(1)?
    } else {
        command.split(' ').next()?
    };
    Some(PathBuf::from(path_str))
}

fn parse_client_name(
    display_name: &str,
    client_name: &str,
) -> Option<(BrowserKind, BrowserChannel)> {
    let name = display_name.to_lowercase();
    let client = client_name.to_lowercase();

    // This is heuristic based.
    let (kind, channel) = if name.contains("chrome") {
        let channel = if name.contains("beta") {
            ChromiumChannel::Beta
        } else if name.contains("dev") {
            ChromiumChannel::Dev
        } else if name.contains("canary") {
            ChromiumChannel::Canary
        } else {
            ChromiumChannel::Stable
        };
        (BrowserKind::Chrome, BrowserChannel::Chromium(channel))
    } else if name.contains("firefox") {
        let channel = if name.contains("developer") {
            FirefoxChannel::Dev
        } else if name.contains("nightly") {
            FirefoxChannel::Nightly
        } else if name.contains("esr") {
            FirefoxChannel::Esr
        } else {
            FirefoxChannel::Stable
        };
        (BrowserKind::Firefox, BrowserChannel::Firefox(channel))
    } else if name.contains("edge") || client.contains("edge") {
        let channel = if name.contains("beta") || client.contains("beta") {
            ChromiumChannel::Beta
        } else if name.contains("dev") || client.contains("dev") {
            ChromiumChannel::Dev
        } else if name.contains("canary") || client.contains("canary") {
            ChromiumChannel::Canary
        } else {
            ChromiumChannel::Stable
        };
        (BrowserKind::Edge, BrowserChannel::Chromium(channel))
    } else if name.contains("brave") || client.contains("brave") {
        let channel = if name.contains("beta") || client.contains("beta") {
            ChromiumChannel::Beta
        } else if name.contains("dev") || name.contains("nightly") || client.contains("nightly") {
            ChromiumChannel::Dev
        } else {
            ChromiumChannel::Stable
        };
        (BrowserKind::Brave, BrowserChannel::Chromium(channel))
    } else if name.contains("opera") {
        let channel = if client.contains("gx") {
            OperaChannel::Gx
        } else if client.contains("beta") {
            OperaChannel::Beta
        } else {
            OperaChannel::Stable
        };
        (BrowserKind::Opera, BrowserChannel::Opera(channel))
    } else if name.contains("vivaldi") || client.contains("vivaldi") {
        (BrowserKind::Vivaldi, BrowserChannel::Single)
    } else if client.contains("tor") || name.contains("tor") {
        (BrowserKind::TorBrowser, BrowserChannel::Single)
    } else {
        return None;
    };

    Some((kind, channel))
}

fn default_prog_id() -> Option<String> {
    const BASE: &str = "Software\\Microsoft\\Windows\\Shell\\Associations\\UrlAssociations";
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    for scheme in ["https", "http"] {
        let path = format!("{}\\{}\\UserChoice", BASE, scheme);
        if let Ok(key) = hkcu.open_subkey(&path) {
            if let Ok::<String, _>(prog_id) = key.get_value("ProgId") {
                if !prog_id.is_empty() {
                    return Some(prog_id);
                }
            }
        }
    }

    None
}

fn browser_info_for_prog_id(prog_id: &str) -> Option<BrowserInfo> {
    const SEARCH_PATH: &str = "SOFTWARE\\Clients\\StartMenuInternet";

    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let base = RegKey::predef(hive);
        if let Ok(clients) = base.open_subkey(SEARCH_PATH) {
            for client_name in clients.enum_keys().filter_map(Result::ok) {
                let reg_path = format!("{}\\{}", SEARCH_PATH, client_name);
                if let Ok(client_key) = base.open_subkey(&reg_path) {
                    if client_matches_prog_id(&client_key, prog_id) {
                        if let Some(info) = create_browser_info(&base, SEARCH_PATH, &client_name) {
                            return Some(info);
                        }
                    }
                }
            }
        }
    }

    None
}

fn client_matches_prog_id(client_key: &RegKey, prog_id: &str) -> bool {
    if let Ok(url_assoc) = client_key.open_subkey("Capabilities\\URLAssociations") {
        for key in ["https", "http"] {
            if let Ok::<String, _>(value) = url_assoc.get_value(key) {
                if value.eq_ignore_ascii_case(prog_id) {
                    return true;
                }
            }
        }
    }

    false
}

fn fallback_system_default(prog_id: &str) -> Option<SystemDefaultBrowser> {
    let command = command_for_prog_id(prog_id)?;
    let executable = parse_command_path(&command)?;

    let display_name = application_display_name(prog_id)
        .or_else(|| {
            executable
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| prog_id.to_string());

    let kind = infer_kind_from_tokens([
        display_name.as_str(),
        prog_id,
        executable.to_string_lossy().as_ref(),
    ]);

    Some(SystemDefaultBrowser {
        identifier: prog_id.to_string(),
        display_name,
        kind,
        path: Some(executable),
    })
}

fn command_for_prog_id(prog_id: &str) -> Option<String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

    let cursors = [
        hkcu.open_subkey(format!(
            "Software\\Classes\\{}\\shell\\open\\command",
            prog_id
        )),
        hkcr.open_subkey(format!("{}\\shell\\open\\command", prog_id)),
    ];

    for cursor in cursors.into_iter().flatten() {
        if let Ok::<String, _>(value) = cursor.get_value("") {
            if !value.is_empty() {
                return Some(value);
            }
        }
    }

    None
}

fn application_display_name(prog_id: &str) -> Option<String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);

    let candidates = [
        hkcu.open_subkey(format!("Software\\Classes\\{}", prog_id)),
        hkcr.open_subkey(prog_id),
    ];

    for key in candidates.into_iter().flatten() {
        if let Ok::<String, _>(value) = key.get_value("ApplicationName") {
            if !value.is_empty() {
                return Some(value);
            }
        }
        if let Ok::<String, _>(value) = key.get_value("ApplicationDescription") {
            if !value.is_empty() {
                return Some(value);
            }
        }
        if let Ok::<String, _>(value) = key.get_value("") {
            if !value.is_empty() {
                return Some(value);
            }
        }
    }

    None
}

fn infer_kind_from_tokens<'a, I>(tokens: I) -> Option<BrowserKind>
where
    I: IntoIterator<Item = &'a str>,
{
    for token in tokens {
        let lowered = token.to_ascii_lowercase();

        if lowered.contains("chromium") {
            return Some(BrowserKind::Chromium);
        }

        if lowered.contains("chrome") {
            return Some(BrowserKind::Chrome);
        }

        if lowered.contains("firefox") {
            return Some(BrowserKind::Firefox);
        }

        if lowered.contains("edge") {
            return Some(BrowserKind::Edge);
        }

        if lowered.contains("brave") {
            return Some(BrowserKind::Brave);
        }

        if lowered.contains("vivaldi") {
            return Some(BrowserKind::Vivaldi);
        }

        if lowered.contains("opera") {
            return Some(BrowserKind::Opera);
        }

        if lowered.contains("tor") {
            return Some(BrowserKind::TorBrowser);
        }
    }

    None
}
