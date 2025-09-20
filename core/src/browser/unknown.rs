use super::{BrowserInfo, LaunchCommand, LaunchOutcome, LaunchTarget, SystemDefaultBrowser};
use crate::filesystem::FileSystem;
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
    #[error("Unable to launch system default browser on this platform")]
    Unsupported,
    #[error("Failed to launch browser: {source}")]
    Spawn {
        #[from]
        source: io::Error,
    },
}

pub fn detect_browsers<F: FileSystem>(_fs: &F) -> Vec<BrowserInfo> {
    Vec::new()
}

/// Returns the system's default browser metadata, if detectable on this platform.
///
/// This is a platform-dependent stub that currently does not detect or return a system
/// default browser and will return `None`.
///
/// # Examples
///
/// ```
/// let sys = system_default_browser();
/// assert!(sys.is_none());
/// ```
pub fn system_default_browser() -> Option<SystemDefaultBrowser> {
    None
}

/// Launch a browser for the given target and URLs.
///
/// This is a thin convenience wrapper around `launch_with_profile` that calls it without
/// profile or window options.
///
/// # Examples
///
/// ```
/// use crate::browser::{launch, LaunchTarget};
///
/// let urls = vec!["https://example.com".to_string()];
/// let _ = launch(LaunchTarget::SystemDefault, &urls);
/// ```
pub fn launch(target: LaunchTarget<'_>, urls: &[String]) -> Result<LaunchOutcome, LaunchError> {
    launch_with_profile(target, urls, None, None)
}

/// Launches the given browser target with the provided URLs, optionally accepting profile and window options.
///
/// If `target` is a specific browser, this will attempt to launch that browser executable with `urls` as arguments
/// and return a `LaunchOutcome` describing the launched command and browser. If `target` is the system default
/// browser, this platform does not support launching the system default and `LaunchError::Unsupported` is returned.
///
/// The `_profile_opts` and `_window_opts` parameters are accepted for future use but are ignored by this implementation.
///
/// # Errors
///
/// Returns a `LaunchError` when:
/// - `LaunchError::NoUrls` if `urls` is empty.
/// - `LaunchError::MissingExecutable` if the chosen browser has no known executable path.
/// - `LaunchError::Unsupported` if `target` is `LaunchTarget::SystemDefault`.
/// - `LaunchError::Spawn` if spawning the browser process fails (propagated from `std::io::Error`).
///
/// # Examples
///
/// ```
/// use crate::core::browser::{launch_with_profile, LaunchTarget};
///
/// let urls = vec!["https://example.com".to_string()];
/// // SystemDefault is unsupported on this platform; this example demonstrates calling the function.
/// let res = launch_with_profile(LaunchTarget::SystemDefault, &urls, None, None);
/// assert!(res.is_err());
/// ```
pub fn launch_with_profile(
    target: LaunchTarget<'_>,
    urls: &[String],
    _profile_opts: Option<&crate::profile::ProfileOptions>,
    _window_opts: Option<&crate::profile::WindowOptions>,
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
            debug!("System default browser launch is unsupported on this platform");
            Err(LaunchError::Unsupported)
        }
    }
}
