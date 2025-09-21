use super::{BrowserInfo, LaunchOutcome, LaunchTarget, SystemDefaultBrowser};
use crate::browser::channels::{
    BrowserChannel, ChromiumChannel, FirefoxChannel, OperaChannel, SafariChannel,
};
use crate::browser::BrowserKind;
use crate::filesystem::FileSystem;
use std::path::PathBuf;
use thiserror::Error;

// Core Foundation and Services imports
use core_foundation::array::{CFArray, CFArrayRef};
use core_foundation::base::TCFType;
use core_foundation::bundle::CFBundle;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation::url::CFURL;

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    fn LSCopyAllHandlersForURLScheme(inURLScheme: CFStringRef) -> CFArrayRef;
    fn LSCopyDefaultHandlerForURLScheme(inURLScheme: CFStringRef) -> CFStringRef;
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

pub fn detect_browsers<F: FileSystem>(_fs: &F) -> Vec<BrowserInfo> {
    let mut browsers = Vec::new();
    let bundle_ids = get_https_handlers();

    for id in bundle_ids {
        if let Some(info) = create_browser_info(&id, _fs) {
            browsers.push(info);
        }
    }
    browsers
}

fn get_https_handlers() -> Vec<String> {
    unsafe {
        let scheme = CFString::new("https");
        let handlers_ref = LSCopyAllHandlersForURLScheme(scheme.as_concrete_TypeRef());
        if handlers_ref.is_null() {
            return Vec::new();
        }
        let handlers: CFArray<CFString> = CFArray::wrap_under_get_rule(handlers_ref as *const _);
        handlers.iter().map(|s| s.to_string()).collect()
    }
}

fn create_browser_info<F: FileSystem>(bundle_id: &str, _fs: &F) -> Option<BrowserInfo> {
    let (kind, channel) = parse_bundle_id(bundle_id)?;

    let app_path = get_app_path_from_bundle_id(bundle_id)?;
    let bundle_url = CFURL::from_path(&app_path, true)?;
    let bundle = CFBundle::new(bundle_url)?;

    let info_dict = bundle.info_dictionary();

    let display_name = if info_dict.contains_key(&CFString::new("CFBundleDisplayName")) {
        info_dict
            .get(CFString::new("CFBundleDisplayName"))
            .downcast::<CFString>()
            .map(|s| s.to_string())
            .unwrap_or_else(|| kind.canonical_name().to_string())
    } else {
        kind.canonical_name().to_string()
    };

    let version = if info_dict.contains_key(&CFString::new("CFBundleShortVersionString")) {
        info_dict
            .get(CFString::new("CFBundleShortVersionString"))
            .downcast::<CFString>()
            .map(|s| s.to_string())
    } else {
        None
    };

    let executable_name = if info_dict.contains_key(&CFString::new("CFBundleExecutable")) {
        info_dict
            .get(CFString::new("CFBundleExecutable"))
            .downcast::<CFString>()
            .map(|s| s.to_string())
    } else {
        None
    };

    let executable_name = executable_name.unwrap_or_else(|| {
        // Fallback: try to extract executable name from app path
        app_path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    });

    let executable_path = app_path.join("Contents/MacOS").join(executable_name);

    Some(BrowserInfo {
        kind,
        channel,
        display_name,
        executable_path,
        version,
        unique_id: bundle_id.to_string(),
        exec_command: None,
    })
}

fn get_app_path_from_bundle_id(bundle_id: &str) -> Option<PathBuf> {
    use std::process::Command;
    let output = Command::new("mdfind")
        .arg(format!("kMDItemCFBundleIdentifier == '{}'", bundle_id))
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let path_str = String::from_utf8(output.stdout).ok()?;
    let first_path = path_str.lines().next()?.trim();
    if first_path.is_empty() {
        None
    } else {
        Some(PathBuf::from(first_path))
    }
}

fn parse_bundle_id(bundle_id: &str) -> Option<(BrowserKind, BrowserChannel)> {
    let lower_id = bundle_id.to_lowercase();

    // Safari
    if lower_id == "com.apple.safari" {
        return Some((
            BrowserKind::Safari,
            BrowserChannel::Safari(SafariChannel::Stable),
        ));
    }
    if lower_id == "com.apple.safari.technologypreview" {
        return Some((
            BrowserKind::Safari,
            BrowserChannel::Safari(SafariChannel::TechnologyPreview),
        ));
    }

    // Firefox
    if lower_id == "org.mozilla.firefox" {
        return Some((
            BrowserKind::Firefox,
            BrowserChannel::Firefox(FirefoxChannel::Stable),
        ));
    }
    if lower_id == "org.mozilla.firefoxdeveloperedition" {
        return Some((
            BrowserKind::Firefox,
            BrowserChannel::Firefox(FirefoxChannel::Dev),
        ));
    }
    if lower_id == "org.mozilla.nightly" {
        return Some((
            BrowserKind::Firefox,
            BrowserChannel::Firefox(FirefoxChannel::Nightly),
        ));
    }

    // Arc
    if lower_id == "company.thebrowser.browser" {
        return Some((BrowserKind::Arc, BrowserChannel::Single));
    }

    // Tor
    if lower_id == "org.torproject.torbrowser" {
        return Some((BrowserKind::TorBrowser, BrowserChannel::Single));
    }

    // Waterfox
    if lower_id == "net.waterfox.waterfox" {
        return Some((BrowserKind::Waterfox, BrowserChannel::Single));
    }

    // Helium
    if lower_id == "net.imput.helium" {
        return Some((BrowserKind::Helium, BrowserChannel::Single));
    }

    // Chromium-based browsers
    let parts: Vec<&str> = lower_id.split('.').collect();
    let company = parts.get(1).copied()?;

    let kind = match company {
        "google" => BrowserKind::Chrome,
        "brave" => BrowserKind::Brave,
        "microsoft" => BrowserKind::Edge,
        "operasoftware" => BrowserKind::Opera,
        "vivaldi" => BrowserKind::Vivaldi,
        "chromium" => BrowserKind::Chromium,
        _ => return None,
    };

    let channel_str = parts.last().copied().unwrap_or("stable");

    let channel = match kind {
        BrowserKind::Chrome | BrowserKind::Brave | BrowserKind::Edge => {
            let ch = match channel_str {
                "beta" => ChromiumChannel::Beta,
                "dev" => ChromiumChannel::Dev,
                "canary" => ChromiumChannel::Canary,
                _ => ChromiumChannel::Stable,
            };
            BrowserChannel::Chromium(ch)
        }
        BrowserKind::Opera => {
            let ch = match channel_str {
                "beta" => OperaChannel::Beta,
                "gx" => OperaChannel::Gx,
                _ => OperaChannel::Stable,
            };
            BrowserChannel::Opera(ch)
        }
        _ => BrowserChannel::Single,
    };

    Some((kind, channel))
}

// Stubbed out functions
pub fn system_default_browser_with_fs<F: FileSystem>(fs: &F) -> Option<SystemDefaultBrowser> {
    let bundle_id = default_handler_for_https()?;

    if let Some(info) = create_browser_info(&bundle_id, fs) {
        let path = info.launch_path().to_path_buf();
        return Some(SystemDefaultBrowser {
            identifier: bundle_id,
            display_name: info.display_name,
            kind: Some(info.kind),
            path: Some(path),
        });
    }

    let display_name = bundle_id.clone();
    Some(SystemDefaultBrowser {
        identifier: bundle_id,
        display_name,
        kind: None,
        path: None,
    })
}

use super::LaunchCommand;
use std::process::{Command, Stdio};
use tracing::debug;

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
            if info.kind == crate::browser::BrowserKind::Safari {
                let mut command = Command::new("open");
                command.arg("-b").arg("com.apple.Safari");

                if let Some(window_opts) = window_opts {
                    if window_opts.new_window {
                        command.arg("--new");
                    }
                }

                command.args(urls);
                command.stdin(Stdio::null());
                command.stdout(Stdio::null());
                command.stderr(Stdio::null());

                let all_args: Vec<String> = command
                    .get_args()
                    .map(|s| s.to_string_lossy().to_string())
                    .collect();
                debug!(program = "open", args = ?all_args, "Launching Safari via open command");
                command.spawn()?;

                let cmd = LaunchCommand {
                    program: PathBuf::from("open"),
                    args: all_args.clone(),
                    display: format!("open {}", all_args.join(" ")),
                    is_system_default: false,
                };

                Ok(LaunchOutcome {
                    browser: Some(info.clone()),
                    system_default: None,
                    command: cmd,
                })
            } else {
                let exec = info.launch_path();

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
        }
        LaunchTarget::SystemDefault => {
            let mut command = Command::new("open");

            if let Some(window_opts) = window_opts {
                if window_opts.new_window {
                    command.arg("--new");
                }
            }

            command.args(urls);
            command.stdin(Stdio::null());
            command.stdout(Stdio::null());
            command.stderr(Stdio::null());

            let all_args: Vec<String> = command
                .get_args()
                .map(|s| s.to_string_lossy().to_string())
                .collect();
            debug!(program = "open", args = ?all_args, "Launching system default browser");
            command.spawn()?;

            let cmd = LaunchCommand {
                program: PathBuf::from("open"),
                args: all_args.clone(),
                display: format!("open {}", all_args.join(" ")),
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

fn default_handler_for_https() -> Option<String> {
    unsafe {
        let scheme = CFString::new("https");
        let handler_ref = LSCopyDefaultHandlerForURLScheme(scheme.as_concrete_TypeRef());
        if handler_ref.is_null() {
            return None;
        }

        let handler = CFString::wrap_under_create_rule(handler_ref);
        let bundle_id = handler.to_string();
        if bundle_id.is_empty() {
            None
        } else {
            Some(bundle_id)
        }
    }
}
