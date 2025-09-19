use crate::browser::{BrowserInfo, BrowserKind};
use dirs;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, warn};

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("Profile '{0}' not found")]
    ProfileNotFound(String),
    #[error("Invalid profile directory: {0}")]
    InvalidDirectory(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Browser does not support profiles: {0}")]
    UnsupportedBrowser(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileInfo {
    pub name: String,
    pub display_name: String,
    pub path: PathBuf,
    pub is_default: bool,
    pub last_used: Option<String>,
    pub browser_kind: BrowserKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileOptions {
    pub profile_type: ProfileType,
    pub custom_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum ProfileType {
    Default,
    Named(String),
    CustomDirectory(PathBuf),
    Temporary(PathBuf),
    Guest,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct WindowOptions {
    pub new_window: bool,
    pub incognito: bool,
    pub kiosk: bool,
}

pub struct ProfileManager;

impl ProfileManager {
    /// Discover profiles for a given browser
    pub fn discover_profiles(browser: &BrowserInfo) -> Result<Vec<ProfileInfo>, ProfileError> {
        Self::discover_profiles_in_directory(browser, None)
    }

    /// Discover profiles for a given browser in a specific directory
    pub fn discover_profiles_in_directory(
        browser: &BrowserInfo,
        custom_base_dir: Option<&Path>,
    ) -> Result<Vec<ProfileInfo>, ProfileError> {
        match browser.kind {
            BrowserKind::Chrome
            | BrowserKind::Edge
            | BrowserKind::Brave
            | BrowserKind::Vivaldi
            | BrowserKind::Arc
            | BrowserKind::Helium
            | BrowserKind::Opera
            | BrowserKind::Chromium => {
                Self::discover_chromium_profiles_in_dir(browser, custom_base_dir)
            }
            BrowserKind::Firefox | BrowserKind::Waterfox => {
                Self::discover_firefox_profiles_in_dir(browser, custom_base_dir)
            }
            BrowserKind::Safari => {
                // Safari doesn't support profiles
                Ok(vec![ProfileInfo {
                    name: "default".to_string(),
                    display_name: "Default".to_string(),
                    path: custom_base_dir.unwrap_or(&PathBuf::new()).to_path_buf(),
                    is_default: true,
                    last_used: None,
                    browser_kind: browser.kind,
                }])
            }
            _ => {
                // Other browsers - assume single profile
                Ok(vec![ProfileInfo {
                    name: "default".to_string(),
                    display_name: "Default".to_string(),
                    path: custom_base_dir.unwrap_or(&PathBuf::new()).to_path_buf(),
                    is_default: true,
                    last_used: None,
                    browser_kind: browser.kind,
                }])
            }
        }
    }

    /// Find a specific profile by name
    pub fn find_profile(
        browser: &BrowserInfo,
        profile_name: &str,
    ) -> Result<ProfileInfo, ProfileError> {
        Self::find_profile_in_directory(browser, profile_name, None)
    }

    /// Find a specific profile by name in a custom directory
    pub fn find_profile_in_directory(
        browser: &BrowserInfo,
        profile_name: &str,
        custom_base_dir: Option<&Path>,
    ) -> Result<ProfileInfo, ProfileError> {
        let profiles = Self::discover_profiles_in_directory(browser, custom_base_dir)?;
        profiles
            .into_iter()
            .find(|p| p.name == profile_name || p.display_name == profile_name)
            .ok_or_else(|| ProfileError::ProfileNotFound(profile_name.to_string()))
    }

    /// Generate launch arguments for profile options
    pub fn generate_profile_args(
        browser: &BrowserInfo,
        profile_opts: &ProfileOptions,
        window_opts: &WindowOptions,
    ) -> Vec<String> {
        let mut args = Vec::new();

        match browser.kind {
            BrowserKind::Chrome
            | BrowserKind::Edge
            | BrowserKind::Brave
            | BrowserKind::Vivaldi
            | BrowserKind::Arc
            | BrowserKind::Helium
            | BrowserKind::Opera
            | BrowserKind::Chromium => {
                args.extend(Self::chromium_profile_args(
                    browser,
                    profile_opts,
                    window_opts,
                ));
            }
            BrowserKind::Firefox | BrowserKind::Waterfox => {
                args.extend(Self::firefox_profile_args(
                    browser,
                    profile_opts,
                    window_opts,
                ));
            }
            BrowserKind::Safari => {
                args.extend(Self::safari_profile_args(profile_opts, window_opts));
            }
            _ => {
                // Other browsers - basic window management only
                args.extend(Self::generic_window_args(window_opts));
            }
        }

        args.extend(profile_opts.custom_args.clone());

        args
    }

    /// Validate and create custom directory if needed
    pub fn prepare_custom_directory(path: &Path) -> Result<PathBuf, ProfileError> {
        let path = path.to_path_buf();

        if !path.exists() {
            fs::create_dir_all(&path).map_err(|e| {
                ProfileError::PermissionDenied(format!(
                    "Cannot create directory {}: {}",
                    path.display(),
                    e
                ))
            })?;
        }

        if !path.is_dir() {
            return Err(ProfileError::InvalidDirectory(format!(
                "{} is not a directory",
                path.display()
            )));
        }
        use std::fs::OpenOptions;
        let test_file = path.join(".pathway_test");
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&test_file)
        {
            Ok(_) => {
                if let Err(e) = fs::remove_file(&test_file) {
                    warn!(
                        "Temporary test file '{}' could not be removed: {}",
                        test_file.display(),
                        e
                    );
                }
            }
            Err(e) => {
                return Err(ProfileError::PermissionDenied(format!(
                    "Cannot write to directory {}: {}",
                    path.display(),
                    e
                )));
            }
        }

        Ok(path)
    }

    /// Create a temporary profile directory
    pub fn create_temp_profile() -> Result<PathBuf, ProfileError> {
        let temp_dir =
            std::env::temp_dir().join(format!("pathway_profile_{}", generate_timestamp_id()));
        fs::create_dir_all(&temp_dir)?;
        Ok(temp_dir)
    }

    fn discover_chromium_profiles_in_dir(
        browser: &BrowserInfo,
        custom_base_dir: Option<&Path>,
    ) -> Result<Vec<ProfileInfo>, ProfileError> {
        let base_dir = match custom_base_dir {
            Some(custom_dir) => custom_dir.to_path_buf(),
            None => Self::get_chromium_base_dir(browser.kind)?,
        };
        let local_state_path = base_dir.join("Local State");

        if !local_state_path.exists() {
            debug!(
                "Local State file not found at {}",
                local_state_path.display()
            );
            return Ok(vec![Self::default_profile(browser.kind)]);
        }

        let local_state_content = fs::read_to_string(&local_state_path)?;
        let local_state: serde_json::Value = serde_json::from_str(&local_state_content)?;

        let mut profiles = Vec::new();

        if let Some(profile_info) = local_state.get("profile").and_then(|p| p.get("info_cache")) {
            if let Some(profile_obj) = profile_info.as_object() {
                for (profile_id, profile_data) in profile_obj {
                    let profile_path = base_dir.join(profile_id);
                    if !profile_path.exists() {
                        continue;
                    }

                    let display_name = profile_data
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or(profile_id)
                        .to_string();

                    let is_default = profile_id == "Default";

                    profiles.push(ProfileInfo {
                        name: profile_id.clone(),
                        display_name,
                        path: profile_path,
                        is_default,
                        last_used: profile_data
                            .get("active_time")
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string()),
                        browser_kind: browser.kind,
                    });
                }
            }
        }

        if profiles.is_empty() {
            let default_path = base_dir.join("Default");
            if default_path.exists() {
                profiles.push(ProfileInfo {
                    name: "Default".to_string(),
                    display_name: "Default".to_string(),
                    path: default_path,
                    is_default: true,
                    last_used: None,
                    browser_kind: browser.kind,
                });
            }
        }

        if profiles.is_empty() {
            profiles.push(Self::default_profile(browser.kind));
        }

        Ok(profiles)
    }

    fn discover_firefox_profiles_in_dir(
        browser: &BrowserInfo,
        custom_base_dir: Option<&Path>,
    ) -> Result<Vec<ProfileInfo>, ProfileError> {
        let base_dir = match custom_base_dir {
            Some(custom_dir) => custom_dir.to_path_buf(),
            None => Self::get_firefox_base_dir()?,
        };
        let profiles_ini_path = base_dir.join("profiles.ini");

        if !profiles_ini_path.exists() {
            debug!("profiles.ini not found at {}", profiles_ini_path.display());
            return Ok(vec![Self::default_profile(browser.kind)]);
        }

        let profiles_ini_content = fs::read_to_string(&profiles_ini_path)?;
        let mut profiles = Vec::new();

        let mut current_profile: Option<HashMap<String, String>> = None;

        for line in profiles_ini_content.lines() {
            let line = line.trim();

            if line.starts_with('[') && line.ends_with(']') {
                if let Some(profile_data) = current_profile.take() {
                    if let Some(profile_info) =
                        Self::parse_firefox_profile(profile_data, &base_dir, browser.kind)
                    {
                        profiles.push(profile_info);
                    }
                }

                if line.starts_with("[Profile") {
                    current_profile = Some(HashMap::new());
                }
            } else if let Some(ref mut profile_data) = current_profile {
                if let Some((key, value)) = line.split_once('=') {
                    profile_data.insert(key.trim().to_string(), value.trim().to_string());
                }
            }
        }

        if let Some(profile_data) = current_profile {
            if let Some(profile_info) =
                Self::parse_firefox_profile(profile_data, &base_dir, browser.kind)
            {
                profiles.push(profile_info);
            }
        }

        if profiles.is_empty() {
            profiles.push(Self::default_profile(browser.kind));
        }

        Ok(profiles)
    }

    fn parse_firefox_profile(
        profile_data: HashMap<String, String>,
        base_dir: &Path,
        browser_kind: BrowserKind,
    ) -> Option<ProfileInfo> {
        let name = profile_data.get("Name")?.clone();
        let is_relative = profile_data
            .get("IsRelative")
            .map(|v| v == "1")
            .unwrap_or(true);

        let path = if is_relative {
            if let Some(path_str) = profile_data.get("Path") {
                base_dir.join(path_str)
            } else {
                return None;
            }
        } else if let Some(path_str) = profile_data.get("Path") {
            PathBuf::from(path_str)
        } else {
            return None;
        };

        if !path.exists() {
            return None;
        }

        let is_default = profile_data
            .get("Default")
            .map(|v| v == "1")
            .unwrap_or(false);

        Some(ProfileInfo {
            name: name.clone(),
            display_name: name,
            path,
            is_default,
            last_used: None,
            browser_kind,
        })
    }

    fn get_chromium_base_dir(browser_kind: BrowserKind) -> Result<PathBuf, ProfileError> {
        let home = dirs::home_dir().ok_or_else(|| {
            ProfileError::InvalidDirectory("Could not determine home directory".to_string())
        })?;

        match browser_kind {
            BrowserKind::Chrome => {
                #[cfg(target_os = "macos")]
                return Ok(home.join("Library/Application Support/Google/Chrome"));
                #[cfg(target_os = "linux")]
                return Ok(home.join(".config/google-chrome"));
                #[cfg(target_os = "windows")]
                return Ok(home.join("AppData/Local/Google/Chrome"));
            }
            BrowserKind::Edge => {
                #[cfg(target_os = "macos")]
                return Ok(home.join("Library/Application Support/Microsoft Edge"));
                #[cfg(target_os = "linux")]
                return Ok(home.join(".config/microsoft-edge"));
                #[cfg(target_os = "windows")]
                return Ok(home.join("AppData/Local/Microsoft/Edge"));
            }
            BrowserKind::Brave => {
                #[cfg(target_os = "macos")]
                return Ok(home.join("Library/Application Support/BraveSoftware/Brave-Browser"));
                #[cfg(target_os = "linux")]
                return Ok(home.join(".config/BraveSoftware/Brave-Browser"));
                #[cfg(target_os = "windows")]
                return Ok(home.join("AppData/Local/BraveSoftware/Brave-Browser"));
            }
            BrowserKind::Vivaldi => {
                #[cfg(target_os = "macos")]
                return Ok(home.join("Library/Application Support/Vivaldi"));
                #[cfg(target_os = "linux")]
                return Ok(home.join(".config/vivaldi"));
                #[cfg(target_os = "windows")]
                return Ok(home.join("AppData/Local/Vivaldi"));
            }
            BrowserKind::Arc => {
                #[cfg(target_os = "macos")]
                return Ok(home.join("Library/Application Support/Arc"));
                #[cfg(target_os = "linux")]
                return Ok(home.join(".config/arc"));
                #[cfg(target_os = "windows")]
                return Ok(home.join("AppData/Local/Arc"));
            }
            BrowserKind::Helium => {
                #[cfg(target_os = "macos")]
                return Ok(home.join("Library/Application Support/net.imput.helium"));
                #[cfg(target_os = "linux")]
                return Ok(home.join(".config/helium"));
                #[cfg(target_os = "windows")]
                return Ok(home.join("AppData/Local/Helium"));
            }
            BrowserKind::Opera => {
                #[cfg(target_os = "macos")]
                return Ok(home.join("Library/Application Support/com.operasoftware.Opera"));
                #[cfg(target_os = "linux")]
                return Ok(home.join(".config/opera"));
                #[cfg(target_os = "windows")]
                return Ok(home.join("AppData/Roaming/Opera Software/Opera Stable"));
            }
            BrowserKind::Chromium => {
                #[cfg(target_os = "macos")]
                return Ok(home.join("Library/Application Support/Chromium"));
                #[cfg(target_os = "linux")]
                return Ok(home.join(".config/chromium"));
                #[cfg(target_os = "windows")]
                return Ok(home.join("AppData/Local/Chromium"));
            }
            _ => Err(ProfileError::UnsupportedBrowser(format!(
                "Profile discovery not supported for {:?}",
                browser_kind
            ))),
        }
    }

    fn get_firefox_base_dir() -> Result<PathBuf, ProfileError> {
        let home = dirs::home_dir().ok_or_else(|| {
            ProfileError::InvalidDirectory("Could not determine home directory".to_string())
        })?;

        #[cfg(target_os = "macos")]
        {
            Ok(home.join("Library/Application Support/Firefox"))
        }
        #[cfg(target_os = "linux")]
        {
            Ok(home.join(".mozilla/firefox"))
        }
        #[cfg(target_os = "windows")]
        {
            Ok(home.join("AppData/Roaming/Mozilla/Firefox"))
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            Err(ProfileError::UnsupportedBrowser(
                "Unsupported platform".to_string(),
            ))
        }
    }

    fn default_profile(browser_kind: BrowserKind) -> ProfileInfo {
        ProfileInfo {
            name: "default".to_string(),
            display_name: "Default".to_string(),
            path: PathBuf::new(),
            is_default: true,
            last_used: None,
            browser_kind,
        }
    }

    fn chromium_profile_args(
        browser: &BrowserInfo,
        profile_opts: &ProfileOptions,
        window_opts: &WindowOptions,
    ) -> Vec<String> {
        let mut args = Vec::new();

        // Profile arguments
        match &profile_opts.profile_type {
            ProfileType::Named(name) => match Self::find_profile(browser, name) {
                Ok(profile_info) => {
                    args.push(format!("--profile-directory={}", profile_info.name));
                    debug!(
                        "Resolved profile '{}' to directory '{}'",
                        name, profile_info.name
                    );
                }
                Err(e) => {
                    args.push(format!("--profile-directory={}", name));
                    warn!(
                        "Profile '{}' not found, using as directory name: {}",
                        name, e
                    );
                }
            },
            ProfileType::CustomDirectory(path) => {
                args.push(format!("--user-data-dir={}", path.display()));
            }
            ProfileType::Temporary(path) => {
                args.push(format!("--user-data-dir={}", path.display()));
            }
            ProfileType::Guest => {
                args.push("--guest".to_string());
            }
            ProfileType::Default => {
                // No additional args needed
            }
        }

        // Window management arguments
        if window_opts.incognito {
            args.push("--incognito".to_string());
        }
        if window_opts.new_window {
            args.push("--new-window".to_string());
        }
        if window_opts.kiosk {
            args.push("--kiosk".to_string());
        }

        args
    }

    fn firefox_profile_args(
        browser: &BrowserInfo,
        profile_opts: &ProfileOptions,
        window_opts: &WindowOptions,
    ) -> Vec<String> {
        let mut args = Vec::new();

        // Profile arguments
        match &profile_opts.profile_type {
            ProfileType::Named(name) => match Self::find_profile(browser, name) {
                Ok(profile_info) => {
                    args.push("-P".to_string());
                    args.push(profile_info.display_name.clone());
                    debug!(
                        "Resolved Firefox profile '{}' to '{}'",
                        name, profile_info.display_name
                    );
                }
                Err(_) => {
                    args.push("-P".to_string());
                    args.push(name.clone());
                    warn!("Firefox profile '{}' not found, using as-is", name);
                }
            },
            ProfileType::CustomDirectory(path) => {
                args.push("--profile".to_string());
                args.push(path.display().to_string());
            }
            ProfileType::Temporary(path) => {
                args.push("--profile".to_string());
                args.push(path.display().to_string());
            }
            ProfileType::Guest => {
                args.push("--private-window".to_string());
            }
            ProfileType::Default => {
                // No additional args needed
            }
        }

        // Window management arguments
        if window_opts.incognito {
            args.push("--private-window".to_string());
        }
        if window_opts.new_window {
            args.push("--new-window".to_string());
        }
        if window_opts.kiosk {
            args.push("--kiosk".to_string());
        }

        args
    }

    fn safari_profile_args(
        _profile_opts: &ProfileOptions,
        window_opts: &WindowOptions,
    ) -> Vec<String> {
        if window_opts.incognito {
            warn!(
                "Safari incognito mode requires AppleScript and is not supported via command line"
            );
        }

        Vec::new()
    }

    fn generic_window_args(window_opts: &WindowOptions) -> Vec<String> {
        let mut args = Vec::new();

        if window_opts.incognito {
            args.push("--private".to_string());
        }

        args
    }
}

fn generate_timestamp_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", timestamp)
}

/// Validate profile options for conflicts and unsupported combinations
pub fn validate_profile_options(
    browser: &BrowserInfo,
    profile_opts: &ProfileOptions,
    window_opts: &WindowOptions,
) -> Result<Vec<String>, ProfileError> {
    let mut warnings = Vec::new();

    if window_opts.incognito && !matches!(profile_opts.profile_type, ProfileType::Default) {
        warnings.push("Incognito mode ignores profile selection".to_string());
    }

    match browser.kind {
        BrowserKind::Safari => {
            match &profile_opts.profile_type {
                ProfileType::Named(_) => {
                    warnings.push("Safari does not support named profiles".to_string());
                }
                ProfileType::CustomDirectory(_) => {
                    warnings
                        .push("Safari does not support custom user data directories".to_string());
                }
                ProfileType::Temporary(_) => {
                    warnings.push("Safari does not support temporary profiles".to_string());
                }
                ProfileType::Guest => {
                    warnings.push("Safari does not support guest mode".to_string());
                }
                ProfileType::Default => {}
            }

            if window_opts.incognito {
                warnings.push("Safari incognito mode requires manual activation (not supported via command line)".to_string());
            }
            if window_opts.kiosk {
                warnings.push("Safari does not support kiosk mode via command line".to_string());
            }
        }

        BrowserKind::Firefox | BrowserKind::Waterfox => {
            if matches!(profile_opts.profile_type, ProfileType::Guest) {
                warnings.push(
                    "Firefox does not support guest mode (use --incognito for private browsing)"
                        .to_string(),
                );
            }
        }

        BrowserKind::Chrome
        | BrowserKind::Edge
        | BrowserKind::Brave
        | BrowserKind::Vivaldi
        | BrowserKind::Arc
        | BrowserKind::Helium
        | BrowserKind::Opera
        | BrowserKind::Chromium => {}

        BrowserKind::TorBrowser => {
            if !matches!(profile_opts.profile_type, ProfileType::Default) {
                warnings.push(
                    "Tor Browser profile options may interfere with anonymity features".to_string(),
                );
            }
            if window_opts.incognito {
                warnings.push("Tor Browser is already private by default".to_string());
            }
        }

        BrowserKind::Other => {
            if !matches!(profile_opts.profile_type, ProfileType::Default) {
                warnings.push(
                    "Profile support unknown for this browser - may not work as expected"
                        .to_string(),
                );
            }
            if window_opts.incognito || window_opts.kiosk {
                warnings.push(
                    "Window options support unknown for this browser - may not work as expected"
                        .to_string(),
                );
            }
        }
    }

    Ok(warnings)
}
