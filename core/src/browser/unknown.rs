use super::{BrowserInfo, LaunchCommand, LaunchOutcome, LaunchTarget, SystemDefaultBrowser};
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

pub fn detect_browsers() -> Vec<BrowserInfo> {
    Vec::new()
}

pub fn system_default_browser() -> Option<SystemDefaultBrowser> {
    None
}

pub fn launch(target: LaunchTarget<'_>, urls: &[String]) -> Result<LaunchOutcome, LaunchError> {
    launch_with_profile(target, urls, None, None)
}

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
