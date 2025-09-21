use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;

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

pub mod channels;
pub mod sources;

pub use self::channels::BrowserChannel;
use self::channels::{ChromiumChannel, FirefoxChannel, OperaChannel, SafariChannel};

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
    DuckDuckGo,
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
            BrowserKind::DuckDuckGo => "duckduckgo",
            BrowserKind::Other => "browser",
        }
    }
}

// Basic browser info without installation source (used for inventory operations)
#[derive(Debug, Clone, Serialize)]
pub struct BasicBrowserInfo {
    pub kind: BrowserKind,
    pub channel: BrowserChannel,
    pub display_name: String,
    pub executable_path: PathBuf,
    pub version: Option<String>,
    // A unique, stable identifier for this specific installation.
    // e.g., macOS bundle ID, Windows registry path, or Linux .desktop file path.
    pub unique_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_command: Option<String>,
}

// Full browser info (installation source removed for performance)
#[derive(Debug, Clone, Serialize)]
pub struct BrowserInfo {
    pub kind: BrowserKind,
    pub channel: BrowserChannel,
    pub display_name: String,
    pub executable_path: PathBuf,
    pub version: Option<String>,
    // A unique, stable identifier for this specific installation.
    // e.g., macOS bundle ID, Windows registry path, or Linux .desktop file path.
    pub unique_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_command: Option<String>,
}

impl From<BrowserInfo> for BasicBrowserInfo {
    fn from(info: BrowserInfo) -> Self {
        BasicBrowserInfo {
            kind: info.kind,
            channel: info.channel,
            display_name: info.display_name,
            executable_path: info.executable_path,
            version: info.version,
            unique_id: info.unique_id,
            exec_command: info.exec_command,
        }
    }
}

impl BrowserInfo {
    pub fn launch_path(&self) -> Option<&std::path::Path> {
        Some(&self.executable_path)
    }

    pub fn alias(&self) -> String {
        let channel_name = self.channel.canonical_name();
        if channel_name == "stable" {
            self.kind.canonical_name().to_string()
        } else {
            format!("{}-{}", self.kind.canonical_name(), channel_name)
        }
    }

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

        // Match kind name
        if normalized == self.kind.canonical_name() {
            return true;
        }

        // Match display name normalized
        if normalized == normalize_token(&self.display_name) {
            return true;
        }

        // Match combination patterns like "chrome-beta", "firefox-nightly"
        let kind_channel = format!(
            "{}-{}",
            self.kind.canonical_name(),
            self.channel.canonical_name()
        );
        if normalized == kind_channel {
            return true;
        }

        false
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
    pub path: Option<PathBuf>,
}

impl SystemDefaultBrowser {
    pub fn fallback() -> Self {
        SystemDefaultBrowser {
            identifier: "system-default".to_string(),
            display_name: "System default".to_string(),
            kind: None,
            path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserInventory {
    pub browsers: Vec<BrowserInfo>,
    pub system_default: SystemDefaultBrowser,
}

pub fn detect_inventory_with_fs<F: crate::filesystem::FileSystem>(fs: &F) -> BrowserInventory {
    let browsers = dedupe_browsers(platform::detect_browsers(fs));
    // TODO: sort
    BrowserInventory {
        browsers,
        system_default: platform::system_default_browser_with_fs(fs)
            .unwrap_or_else(SystemDefaultBrowser::fallback),
    }
}

pub fn detect_inventory() -> BrowserInventory {
    detect_inventory_with_fs(&crate::filesystem::RealFileSystem)
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
pub fn launch(target: LaunchTarget<'_>, urls: &[String]) -> Result<LaunchOutcome, LaunchError> {
    platform::launch(target, urls)
}

/// Launches a browser target with the given URLs, optionally specifying profile and window options.
pub fn launch_with_profile(
    target: LaunchTarget<'_>,
    urls: &[String],
    profile_opts: Option<&crate::profile::ProfileOptions>,
    window_opts: Option<&crate::profile::WindowOptions>,
) -> Result<LaunchOutcome, LaunchError> {
    platform::launch_with_profile(target, urls, profile_opts, window_opts)
}

pub fn find_browser<'a>(
    browsers: &'a [BrowserInfo],
    token: &str,
    channel: Option<BrowserChannel>,
) -> Option<&'a BrowserInfo> {
    let normalized = normalize_token(token);

    // Find browsers matching the token and channel
    browsers
        .iter()
        .find(|browser| browser.matches_normalized_token(&normalized, channel))
}

pub fn available_tokens(browsers: &[BrowserInfo]) -> Vec<String> {
    let mut tokens = Vec::new();

    for browser in browsers {
        // Add kind name
        tokens.push(browser.kind.canonical_name().to_string());

        // Add kind-channel combination
        let kind_channel = format!(
            "{}-{}",
            browser.kind.canonical_name(),
            browser.channel.canonical_name()
        );
        tokens.push(kind_channel);
    }

    tokens.sort();
    tokens.dedup();
    tokens
}

fn dedupe_browsers(browsers: Vec<BrowserInfo>) -> Vec<BrowserInfo> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();

    for browser in browsers {
        let signature = browser
            .exec_command
            .clone()
            .unwrap_or_else(|| browser.executable_path.to_string_lossy().to_string());

        let key = format!(
            "{}|{}|{}",
            browser.kind.canonical_name(),
            browser.channel.canonical_name(),
            signature
        );

        if seen.insert(key) {
            unique.push(browser);
        }
    }

    unique
}

pub fn default_channel_priority(channel: &BrowserChannel) -> u8 {
    match channel {
        BrowserChannel::Chromium(ch) => match ch {
            ChromiumChannel::Stable => 0,
            ChromiumChannel::Beta => 1,
            ChromiumChannel::Dev => 2,
            ChromiumChannel::Canary => 3,
        },
        BrowserChannel::Firefox(ch) => match ch {
            FirefoxChannel::Stable => 0,
            FirefoxChannel::Esr => 1,
            FirefoxChannel::Beta => 2,
            FirefoxChannel::Dev => 3,
            FirefoxChannel::Nightly => 4,
        },
        BrowserChannel::Opera(ch) => match ch {
            OperaChannel::Stable => 0,
            OperaChannel::Beta => 1,
            OperaChannel::Gx => 2,
        },
        BrowserChannel::Safari(ch) => match ch {
            SafariChannel::Stable => 0,
            SafariChannel::TechnologyPreview => 1,
        },
        BrowserChannel::Single => 0,
    }
}
