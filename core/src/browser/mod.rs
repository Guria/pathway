use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as platform;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
mod unknown;
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
use unknown as platform;

pub use platform::LaunchError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BrowserKind {
    Chrome,
    Firefox,
    Safari,
    Edge,
    Brave,
    Arc,
    Helium,
    Vivaldi,
    Opera,
    TorBrowser,
    Chromium,
    Waterfox,
    Other,
}

impl BrowserKind {
    pub fn canonical_name(self) -> &'static str {
        match self {
            BrowserKind::Chrome => "chrome",
            BrowserKind::Firefox => "firefox",
            BrowserKind::Safari => "safari",
            BrowserKind::Edge => "edge",
            BrowserKind::Brave => "brave",
            BrowserKind::Arc => "arc",
            BrowserKind::Helium => "helium",
            BrowserKind::Vivaldi => "vivaldi",
            BrowserKind::Opera => "opera",
            BrowserKind::TorBrowser => "tor",
            BrowserKind::Chromium => "chromium",
            BrowserKind::Waterfox => "waterfox",
            BrowserKind::Other => "browser",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum BrowserChannel {
    #[default]
    Stable,
    Beta,
    Dev,
    Canary,
    Nightly,
    Unknown,
}

impl BrowserChannel {
    pub fn canonical_name(self) -> &'static str {
        match self {
            BrowserChannel::Stable => "stable",
            BrowserChannel::Beta => "beta",
            BrowserChannel::Dev => "dev",
            BrowserChannel::Canary => "canary",
            BrowserChannel::Nightly => "nightly",
            BrowserChannel::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserInfo {
    pub id: String,
    pub cli_name: String,
    pub display_name: String,
    pub kind: BrowserKind,
    pub channel: BrowserChannel,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl BrowserInfo {
    pub fn matches_token(&self, token: &str, channel: Option<BrowserChannel>) -> bool {
        let normalized = normalize_token(token);
        self.matches_normalized_token(&normalized, channel)
    }

    pub fn matches_normalized_token(
        &self,
        normalized: &str,
        channel: Option<BrowserChannel>,
    ) -> bool {
        if normalized.is_empty() {
            return false;
        }

        if let Some(requested) = channel {
            if requested != self.channel {
                return false;
            }
        }

        if normalized == self.cli_name {
            return true;
        }

        if normalized == self.kind.canonical_name() {
            return true;
        }

        self.aliases.iter().any(|alias| alias == normalized)
    }

    pub fn launch_path(&self) -> Option<&Path> {
        self.executable.as_deref()
    }
}

fn normalize_token(token: &str) -> String {
    token.trim().to_ascii_lowercase().replace([' ', '_'], "-")
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemDefaultBrowser {
    pub identifier: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<BrowserKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<BrowserChannel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}

impl SystemDefaultBrowser {
    pub fn fallback() -> Self {
        SystemDefaultBrowser {
            identifier: "system-default".to_string(),
            display_name: "System default".to_string(),
            kind: None,
            channel: None,
            path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserInventory {
    pub browsers: Vec<BrowserInfo>,
    pub system_default: SystemDefaultBrowser,
}

pub fn detect_inventory() -> BrowserInventory {
    let mut browsers = platform::detect_browsers(&crate::filesystem::RealFileSystem);
    deduplicate(&mut browsers);
    browsers.sort_by(|a, b| {
        (
            a.kind.canonical_name(),
            a.channel.canonical_name(),
            &a.display_name,
        )
            .cmp(&(
                b.kind.canonical_name(),
                b.channel.canonical_name(),
                &b.display_name,
            ))
    });

    BrowserInventory {
        browsers,
        system_default: platform::system_default_browser()
            .unwrap_or_else(SystemDefaultBrowser::fallback),
    }
}

fn deduplicate(browsers: &mut Vec<BrowserInfo>) {
    let mut seen: HashSet<String> = HashSet::new();
    browsers.retain(|browser| seen.insert(browser.cli_name.clone()));
}

#[derive(Debug, Clone, Serialize)]
pub struct LaunchCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub display: String,
    pub is_system_default: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LaunchOutcome {
    pub browser: Option<BrowserInfo>,
    pub system_default: Option<SystemDefaultBrowser>,
    pub command: LaunchCommand,
}

#[derive(Debug, Clone)]
pub enum LaunchTarget<'a> {
    Browser(&'a BrowserInfo),
    SystemDefault,
}

/// Launches the given URLs using the specified launch target.
///
/// Returns a `LaunchOutcome` on success or a platform-specific `LaunchError` on failure.
///
/// # Examples
///
/// ```no_run
/// use pathway::{launch, LaunchTarget, detect_inventory};
///
/// let inventory = detect_inventory();
/// let urls = vec!["https://example.com".to_string()];
/// let outcome = launch(LaunchTarget::SystemDefault, &urls);
/// match outcome {
///     Ok(o) => println!("Launched: {}", o.command.display),
///     Err(e) => eprintln!("Launch failed: {}", e),
/// }
/// ```
pub fn launch(target: LaunchTarget<'_>, urls: &[String]) -> Result<LaunchOutcome, LaunchError> {
    platform::launch(target, urls)
}

/// Launches a browser target with the given URLs, optionally specifying profile and window options.
///
/// This is a thin wrapper that delegates to the platform-specific `launch_with_profile` implementation.
///
/// # Parameters
///
/// - `target`: the launch target (a specific browser or the system default).
/// - `urls`: list of URL strings to open.
/// - `profile_opts`: optional profile-related options (e.g., profile name or path).
/// - `window_opts`: optional window-related options (e.g., new window or focus behavior).
///
/// # Returns
///
/// Returns `Ok(LaunchOutcome)` on success, or `Err(LaunchError)` if the platform-specific launch failed.
///
/// # Examples
///
/// ```no_run
/// use pathway::{launch_with_profile, LaunchTarget, detect_inventory, ProfileOptions, ProfileType, WindowOptions};
///
/// let inventory = detect_inventory();
/// let urls = vec!["https://example.com".to_string()];
/// let profile_opts = ProfileOptions {
///     profile_type: ProfileType::Default,
///     custom_args: Vec::new(),
/// };
/// let window_opts = WindowOptions::default();
/// let outcome = launch_with_profile(LaunchTarget::SystemDefault, &urls, Some(&profile_opts), Some(&window_opts));
/// ```
pub fn launch_with_profile(
    target: LaunchTarget<'_>,
    urls: &[String],
    profile_opts: Option<&crate::profile::ProfileOptions>,
    window_opts: Option<&crate::profile::WindowOptions>,
) -> Result<LaunchOutcome, LaunchError> {
    platform::launch_with_profile(target, urls, profile_opts, window_opts)
}

/// Finds the first browser in `browsers` that matches `token`, optionally constrained to `channel`.
///
/// The `token` is normalized (trimmed, lowercased, spaces/underscores → dashes) before matching.
/// Matching prioritizes exact CLI name matches first, then falls back to kind/alias matching.
/// If `channel` is specified, only browsers with that channel are considered.
///
/// Returns a reference to the first matching `BrowserInfo`, or `None` if no match is found.
///
/// # Examples
///
/// ```no_run
/// use pathway::{find_browser, detect_inventory, BrowserChannel};
///
/// let inventory = detect_inventory();
///
/// // Find Chrome stable
/// let chrome = find_browser(&inventory.browsers, "chrome", None);
///
/// // Find Chrome canary specifically  
/// let canary = find_browser(&inventory.browsers, "chrome", Some(BrowserChannel::Canary));
///
/// // Find by alias
/// let chrome_alias = find_browser(&inventory.browsers, "google-chrome", None);
/// ```
pub fn find_browser<'a>(
    browsers: &'a [BrowserInfo],
    token: &str,
    channel: Option<BrowserChannel>,
) -> Option<&'a BrowserInfo> {
    let normalized = normalize_token(token);

    // First, try to find an exact CLI name match
    if let Some(browser) = browsers.iter().find(|browser| {
        if let Some(requested) = channel {
            if requested != browser.channel {
                return false;
            }
        }
        normalized == browser.cli_name
    }) {
        return Some(browser);
    }

    // Then fall back to kind/alias matching using the already normalized token
    browsers
        .iter()
        .find(|browser| browser.matches_normalized_token(&normalized, channel))
}

pub fn available_tokens(browsers: &[BrowserInfo]) -> Vec<String> {
    browsers
        .iter()
        .map(|browser| browser.cli_name.clone())
        .collect()
}
