use crate::browser::{BrowserInfo, BrowserKind};
use dirs_next;
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
    /// Discover available profiles for the specified browser.
    ///
    /// Returns a list of discovered ProfileInfo entries or a ProfileError if discovery fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pathway::{ProfileManager, BrowserInfo, BrowserKind, BrowserChannel};
    ///
    /// // Example: discover profiles for a browser
    /// // let browser_info = /* construct BrowserInfo */;
    /// // let profiles = ProfileManager::discover_profiles(&browser_info).unwrap();
    /// // assert!(profiles.iter().all(|p| !p.name.is_empty()));
    /// ```
    pub fn discover_profiles(browser: &BrowserInfo) -> Result<Vec<ProfileInfo>, ProfileError> {
        Self::discover_profiles_in_directory(browser, None)
    }

    /// Discover profiles for `browser` using an optional custom base directory.
    ///
    /// Returns a vector of discovered `ProfileInfo` entries for the given browser:
    /// - For Chromium-family browsers (Chrome, Edge, Brave, Vivaldi, Arc, Helium, Opera, Chromium)
    ///   this delegates to Chromium-specific discovery and may return multiple profiles.
    /// - For Firefox and Waterfox this delegates to Firefox-specific discovery and may return multiple profiles.
    /// - For Safari and unknown/other browsers this returns a single default profile whose path is the
    ///   provided `custom_base_dir` (if any) or an empty `PathBuf` otherwise.
    ///
    /// `custom_base_dir` can be used to override the platform default profile directory during discovery.
    /// The function returns a `ProfileError` if underlying I/O or parsing operations fail.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pathway::{ProfileManager, BrowserInfo};
    ///
    /// // Example: discover profiles in custom directory
    /// // let browser = /* construct BrowserInfo */;
    /// // let profiles = ProfileManager::discover_profiles_in_directory(&browser, None).unwrap();
    /// // assert!(!profiles.is_empty());
    /// ```
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
                let path = match custom_base_dir {
                    Some(dir) => dir.to_path_buf(),
                    None => Self::get_default_browser_dir(browser.kind)?,
                };
                Ok(vec![ProfileInfo {
                    name: "default".to_string(),
                    display_name: "Default".to_string(),
                    path,
                    is_default: true,
                    last_used: None,
                    browser_kind: browser.kind,
                }])
            }
            _ => {
                // Other browsers - assume single profile
                let path = match custom_base_dir {
                    Some(dir) => dir.to_path_buf(),
                    None => Self::get_default_browser_dir(browser.kind)?,
                };
                Ok(vec![ProfileInfo {
                    name: "default".to_string(),
                    display_name: "Default".to_string(),
                    path,
                    is_default: true,
                    last_used: None,
                    browser_kind: browser.kind,
                }])
            }
        }
    }

    /// Finds a browser profile by name for the given browser.
    ///
    /// Searches the browser’s default profile directory and returns the first
    /// profile whose internal name or display name matches `profile_name`.
    /// Returns `ProfileError::ProfileNotFound` if no matching profile exists,
    /// and may propagate discovery errors encountered while scanning.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Assume you have a BrowserInfo for Chrome.
    /// let browser: BrowserInfo = get_chrome_browser_info();
    ///
    /// // Look up a profile by name (e.g., "Default" or a custom profile name).
    /// let profile = ProfileManager::find_profile(&browser, "Default")?;
    /// assert!(profile.is_default);
    /// ```
    pub fn find_profile(
        browser: &BrowserInfo,
        profile_name: &str,
    ) -> Result<ProfileInfo, ProfileError> {
        Self::find_profile_in_directory(browser, profile_name, None)
    }

    /// Find and return a profile whose `name` or `display_name` matches `profile_name`.
    ///
    /// This performs profile discovery (optionally under `custom_base_dir`) and searches the
    /// resulting profiles for an exact match against either `ProfileInfo::name` or
    /// `ProfileInfo::display_name`. If found, the matching `ProfileInfo` is returned.
    ///
    /// Errors:
    /// - Returns `ProfileError::ProfileNotFound` if no matching profile is found.
    /// - Propagates errors returned by `discover_profiles_in_directory` (I/O, parsing, etc.).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pathway::{ProfileManager, BrowserInfo};
    /// use std::path::Path;
    ///
    /// // Example: find profile in custom directory
    /// // let result = ProfileManager::find_profile_in_directory(&browser, "Default", None);
    /// // match result {
    /// //     Ok(profile) => println!("Found profile at {}", profile.path.display()),
    /// //     Err(e) => eprintln!("Profile lookup failed: {:?}", e),
    /// // }
    /// ```
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

    /// Build command-line arguments to launch a browser according to the selected profile and window options.
    ///
    /// Chooses a browser-specific argument builder (Chromium-family, Firefox, Safari) based on `browser.kind`,
    /// then appends any custom arguments from `profile_opts.custom_args`. Returns the full argument list to
    /// pass to the browser executable.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pathway::{ProfileManager, ProfileOptions, ProfileType, WindowOptions, BrowserInfo};
    ///
    /// // Example: generate profile arguments
    /// // let profile_opts = ProfileOptions { profile_type: ProfileType::Default, custom_args: vec![] };
    /// // let window_opts = WindowOptions::default();
    /// // let args = ProfileManager::generate_profile_args(&browser, &profile_opts, &window_opts);
    /// ```
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

    /// Ensure a path exists and is writable, creating the directory if necessary.
    ///
    /// This function:
    /// - Creates the directory and any missing parent directories if the path does not exist.
    /// - Verifies the path is a directory.
    /// - Verifies the process can create and remove a small temporary file inside the directory to confirm write access.
    ///
    /// Returns the canonical PathBuf (owned) on success.
    ///
    /// Errors:
    /// - Returns `ProfileError::PermissionDenied` if the directory cannot be created or is not writable.
    /// - Returns `ProfileError::InvalidDirectory` if the path exists but is not a directory.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::env;
    /// use std::fs;
    /// use std::path::Path;
    /// use pathway::ProfileManager;
    ///
    /// // Example: prepare custom directory
    /// // let dir = env::temp_dir().join("pathway_example_dir");
    /// // let result = ProfileManager::prepare_custom_directory(Path::new(&dir));
    /// // assert!(result.is_ok());
    /// ```
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
            .create(true)
            .truncate(true)
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

    /// Create a new unique temporary profile directory and return its path.
    ///
    /// The directory is created under the system temporary directory with a
    /// name prefixed by `pathway_profile_` and a timestamp-based identifier.
    /// Returns the created directory path or a `ProfileError` if creation fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pathway::ProfileManager;
    ///
    /// // Example: create temporary profile
    /// // let dir = ProfileManager::create_temp_profile().expect("failed to create temp profile");
    /// // assert!(dir.exists() && dir.is_dir());
    /// ```
    pub fn create_temp_profile() -> Result<PathBuf, ProfileError> {
        let temp_dir =
            std::env::temp_dir().join(format!("pathway_profile_{}", generate_timestamp_id()));
        fs::create_dir_all(&temp_dir)?;
        Ok(temp_dir)
    }

    /// Discover Chromium-based browser profiles by reading the "Local State" file in
    /// the browser's user data directory (or a provided custom base directory).
    ///
    /// Returns a Vec of ProfileInfo for each profile found. Behavior:
    /// - If a `custom_base_dir` is provided, it is used as the base user-data directory;
    ///   otherwise the platform-specific chromium base directory for `browser.kind` is used.
    /// - If the "Local State" file is missing, returns a single default ProfileInfo.
    /// - If "Local State" contains a `profile.info_cache` object, each entry that has a
    ///   corresponding profile directory under the base directory becomes a ProfileInfo
    ///   (name, display_name, path, is_default, last_used).
    /// - If no profiles are discovered but a "Default" directory exists, a Default profile
    ///   entry is returned.
    /// - If nothing can be discovered, returns a default ProfileInfo as a fallback.
    ///
    /// Returns:
    /// - Ok(`Vec<ProfileInfo>`) on success.
    /// - Err(ProfileError) on IO or JSON parse errors or if the browser kind is unsupported
    ///   when resolving the chromium base directory.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pathway::{ProfileManager, BrowserInfo, BrowserKind};
    ///
    /// // Example: discover Chromium profiles
    /// // let profiles = ProfileManager::discover_chromium_profiles_in_dir(&browser, None).unwrap();
    /// // assert!(!profiles.is_empty());
    /// ```
    pub fn discover_chromium_profiles_in_dir(
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

    /// Discover Firefox profiles by reading a `profiles.ini` file in the Firefox base directory (or a provided custom directory).
    ///
    /// Returns a list of discovered `ProfileInfo` entries. If `profiles.ini` is missing or no valid profiles are parsed,
    /// a single default profile (from `Self::default_profile`) is returned. The function reads and parses `profiles.ini`
    /// sections, converts each profile section via `Self::parse_firefox_profile`, and preserves the discovery order.
    ///
    /// # Errors
    ///
    /// Returns a `ProfileError` if the base directory cannot be resolved or if I/O/JSON operations fail while reading the file.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// // Example usage (types shown for clarity; construct `browser` according to your codebase):
    /// // let browser = BrowserInfo { kind: BrowserKind::Firefox, /* ... */ };
    /// // let profiles = ProfileManager::discover_firefox_profiles_in_dir(&browser, Some(Path::new("/custom/firefox"))).unwrap();
    /// ```
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

    /// Parse a Firefox `profiles.ini` profile entry into a `ProfileInfo`.
    ///
    /// Returns `None` when required fields are missing or the resolved profile path does not exist.
    /// - Treats `IsRelative=1` (or missing) as joining `Path` to `base_dir`; when `IsRelative=0` `Path` is used as absolute.
    /// - `Name` becomes both `name` and `display_name`.
    /// - `Default=1` sets `is_default = true`; otherwise false.
    /// - `last_used` is not populated by this parser.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::collections::HashMap;
    /// use pathway::{ProfileManager, BrowserKind};
    ///
    /// // Example: parse Firefox profile data
    /// // let mut data = HashMap::new();
    /// // data.insert("Name".to_string(), "TestProfile".to_string());
    /// // data.insert("IsRelative".to_string(), "1".to_string());
    /// // data.insert("Path".to_string(), "test.profile".to_string());
    /// // data.insert("Default".to_string(), "1".to_string());
    /// //
    /// // let info = ProfileManager::parse_firefox_profile(data, &base_dir, BrowserKind::Firefox);
    /// ```
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

    /// Returns the platform-specific user data base directory for Chromium-family browsers.
    ///
    /// Given a `BrowserKind` for a Chromium-based browser (Chrome, Edge, Brave, Vivaldi, Arc,
    /// Helium, Opera, Chromium), this returns the expected base profile directory for the current
    /// operating system (macOS, Linux, Windows). The returned path is suitable for locating the
    /// browser's profile subdirectories (e.g. `Default`, `Profile 1`) or for use as `--user-data-dir`.
    ///
    /// Returns `Err(ProfileError::InvalidDirectory(_))` if the user's home directory cannot be
    /// determined, or `Err(ProfileError::UnsupportedBrowser(_))` if `browser_kind` is not a
    /// supported Chromium-family browser.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pathway::{ProfileManager, BrowserKind};
    ///
    /// // Example: get Chromium base directory
    /// // let dir = ProfileManager::get_chromium_base_dir(BrowserKind::Chrome).expect("expected to resolve base dir");
    /// // assert!(dir.is_absolute());
    /// ```
    fn get_chromium_base_dir(browser_kind: BrowserKind) -> Result<PathBuf, ProfileError> {
        let home = dirs_next::home_dir().ok_or_else(|| {
            ProfileError::InvalidDirectory("Could not determine home directory".to_string())
        })?;

        match browser_kind {
            BrowserKind::Chrome => {
                #[cfg(target_os = "macos")]
                return Ok(home.join("Library/Application Support/Google/Chrome"));
                #[cfg(target_os = "linux")]
                return Ok(home.join(".config/google-chrome"));
                #[cfg(target_os = "windows")]
                return Ok(home.join("AppData/Local/Google/Chrome/User Data"));
            }
            BrowserKind::Edge => {
                #[cfg(target_os = "macos")]
                return Ok(home.join("Library/Application Support/Microsoft Edge"));
                #[cfg(target_os = "linux")]
                return Ok(home.join(".config/microsoft-edge"));
                #[cfg(target_os = "windows")]
                return Ok(home.join("AppData/Local/Microsoft/Edge/User Data"));
            }
            BrowserKind::Brave => {
                #[cfg(target_os = "macos")]
                return Ok(home.join("Library/Application Support/BraveSoftware/Brave-Browser"));
                #[cfg(target_os = "linux")]
                return Ok(home.join(".config/BraveSoftware/Brave-Browser"));
                #[cfg(target_os = "windows")]
                return Ok(home.join("AppData/Local/BraveSoftware/Brave-Browser/User Data"));
            }
            BrowserKind::Vivaldi => {
                #[cfg(target_os = "macos")]
                return Ok(home.join("Library/Application Support/Vivaldi"));
                #[cfg(target_os = "linux")]
                return Ok(home.join(".config/vivaldi"));
                #[cfg(target_os = "windows")]
                return Ok(home.join("AppData/Local/Vivaldi/User Data"));
            }
            BrowserKind::Arc => {
                #[cfg(target_os = "macos")]
                return Ok(home.join("Library/Application Support/Arc"));
                #[cfg(target_os = "linux")]
                return Ok(home.join(".config/arc"));
                #[cfg(target_os = "windows")]
                return Ok(home.join("AppData/Local/Arc/User Data"));
            }
            BrowserKind::Helium => {
                #[cfg(target_os = "macos")]
                return Ok(home.join("Library/Application Support/net.imput.helium"));
                #[cfg(target_os = "linux")]
                return Ok(home.join(".config/helium"));
                #[cfg(target_os = "windows")]
                return Ok(home.join("AppData/Local/Helium/User Data"));
            }
            BrowserKind::Opera => {
                #[cfg(target_os = "macos")]
                return Ok(home.join("Library/Application Support/com.operasoftware.Opera"));
                #[cfg(target_os = "linux")]
                return Ok(home.join(".config/opera"));
                #[cfg(target_os = "windows")]
                return Ok(home.join("AppData/Roaming/Opera Software/Opera Stable/User Data"));
            }
            BrowserKind::Chromium => {
                #[cfg(target_os = "macos")]
                return Ok(home.join("Library/Application Support/Chromium"));
                #[cfg(target_os = "linux")]
                return Ok(home.join(".config/chromium"));
                #[cfg(target_os = "windows")]
                return Ok(home.join("AppData/Local/Chromium/User Data"));
            }
            _ => Err(ProfileError::UnsupportedBrowser(format!(
                "Profile discovery not supported for {:?}",
                browser_kind
            ))),
        }
    }

    /// Returns the platform-specific base directory for Firefox profiles under the current user's home directory.
    ///
    /// On macOS this is `~/Library/Application Support/Firefox`, on Linux `~/.mozilla/firefox`,
    /// and on Windows `~/AppData/Roaming/Mozilla/Firefox`. If the user's home directory cannot be
    /// determined the function returns `ProfileError::InvalidDirectory`. On unsupported platforms
    /// it returns `ProfileError::UnsupportedBrowser`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pathway::ProfileManager;
    ///
    /// // Example: get Firefox base directory
    /// // let base = ProfileManager::get_firefox_base_dir().expect("failed to locate Firefox base directory");
    /// // println!("{}", base.display());
    /// ```
    fn get_firefox_base_dir() -> Result<PathBuf, ProfileError> {
        let home = dirs_next::home_dir().ok_or_else(|| {
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

    /// Get the default browser directory for a given browser kind.
    ///
    /// Returns the platform-appropriate default profile/config directory for the specified browser.
    /// For Safari on macOS, this returns the user's Library/Safari path.
    /// For Chromium-based browsers, this delegates to get_chromium_base_dir.
    /// For Firefox-based browsers, this delegates to get_firefox_base_dir.
    /// For unknown browsers or unsupported platforms, this returns an error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pathway::{ProfileManager, BrowserKind};
    ///
    /// // Example: get Safari default directory on macOS
    /// // let dir = ProfileManager::get_default_browser_dir(BrowserKind::Safari).expect("expected Safari dir");
    /// // assert!(dir.to_string_lossy().contains("Library/Safari"));
    /// ```
    fn get_default_browser_dir(browser_kind: BrowserKind) -> Result<PathBuf, ProfileError> {
        match browser_kind {
            // Chromium-based browsers
            BrowserKind::Chrome
            | BrowserKind::Edge
            | BrowserKind::Brave
            | BrowserKind::Vivaldi
            | BrowserKind::Arc
            | BrowserKind::Helium
            | BrowserKind::Opera
            | BrowserKind::Chromium => Self::get_chromium_base_dir(browser_kind),

            // Firefox-based browsers
            BrowserKind::Firefox | BrowserKind::Waterfox => Self::get_firefox_base_dir(),

            // Safari (macOS only)
            BrowserKind::Safari => {
                #[cfg(target_os = "macos")]
                {
                    let home = dirs_next::home_dir().ok_or_else(|| {
                        ProfileError::InvalidDirectory(
                            "Could not determine home directory".to_string(),
                        )
                    })?;
                    Ok(home.join("Library/Safari"))
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Err(ProfileError::UnsupportedBrowser(
                        "Safari is only supported on macOS".to_string(),
                    ))
                }
            }

            // Tor Browser - has its own directory structure
            BrowserKind::TorBrowser => {
                let home = dirs_next::home_dir().ok_or_else(|| {
                    ProfileError::InvalidDirectory("Could not determine home directory".to_string())
                })?;

                #[cfg(target_os = "macos")]
                {
                    Ok(home.join("Library/Application Support/TorBrowser-Data"))
                }
                #[cfg(target_os = "linux")]
                {
                    Ok(home.join(".local/share/torbrowser"))
                }
                #[cfg(target_os = "windows")]
                {
                    Ok(home.join("AppData/Local/TorBrowser"))
                }
                #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
                {
                    Err(ProfileError::UnsupportedBrowser(
                        "Tor Browser not supported on this platform".to_string(),
                    ))
                }
            }

            // Unknown browsers
            BrowserKind::Other => Err(ProfileError::UnsupportedBrowser(
                "Cannot determine default directory for unknown browser".to_string(),
            )),
        }
    }

    /// Construct a default ProfileInfo for the given browser kind.
    ///
    /// Returns a ProfileInfo representing the canonical "default" profile: name "default",
    /// display name "Default", an empty path, marked as the default profile, and no last-used timestamp.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pathway::{ProfileManager, BrowserKind};
    ///
    /// // Example: create default profile info
    /// // let info = ProfileManager::default_profile(BrowserKind::Chrome);
    /// // assert_eq!(info.name, "default");
    /// // assert!(info.is_default);
    /// ```
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

    /// Build command-line arguments for launching Chromium-family browsers according to
    /// the selected profile and requested window options.
    ///
    /// - `ProfileType::Named(name)` is resolved via `find_profile`; when found the resolved
    ///   profile's `name` is passed as `--profile-directory=<name>`. If resolution fails the
    ///   supplied `name` is used as the directory name.
    /// - `ProfileType::CustomDirectory` and `ProfileType::Temporary` set `--user-data-dir=<path>`.
    /// - `ProfileType::Guest` adds `--guest`. `ProfileType::Default` adds no profile-specific flags.
    /// - Window options add `--incognito`, `--new-window`, and `--kiosk` when enabled.
    ///
    /// Returns the assembled argument list (may be empty for defaults).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pathway::{BrowserInfo, BrowserKind, BrowserChannel, ProfileOptions, ProfileType, WindowOptions};
    /// use std::path::PathBuf;
    ///
    /// // This would be called internally by ProfileManager
    /// // Example shows the expected behavior for Chromium browsers
    /// ```
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
                    args.push(format!("--profile-directory={}", &profile_info.name));
                    debug!(
                        "Resolved profile '{}' to directory '{}'",
                        name, &profile_info.name
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

    /// Build command-line arguments for launching Firefox-family browsers based on the selected profile and window options.
    ///
    /// The function maps ProfileType to Firefox CLI flags:
    /// - `Named(name)`: resolves the named profile; if found the profile's display name is passed with `-P`, otherwise the provided name is used.
    /// - `CustomDirectory(path)` / `Temporary(path)`: passed as `--profile <path>`.
    /// - `Guest`: requests a private window with `--private-window`.
    ///
    /// WindowOptions set the window-level flags: `--private-window`, `--new-window`, and `--kiosk` are appended when requested.
    ///
    /// Returns a `Vec<String>` containing the arguments to append to a Firefox launch command.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pathway::{ProfileOptions, ProfileType, WindowOptions, BrowserInfo};
    ///
    /// // Example: generate Firefox profile arguments
    /// // let profile_opts = ProfileOptions { profile_type: ProfileType::Named("default".into()), custom_args: vec![] };
    /// // let window_opts = WindowOptions { new_window: true, incognito: false, kiosk: false };
    /// // let args = ProfileManager::firefox_profile_args(&browser, &profile_opts, &window_opts);
    /// ```
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

    /// Build command-line arguments for launching Safari according to the given options.
    ///
    /// For Safari this function currently does not produce any launch arguments.
    /// If `window_opts.incognito` is true a warning is emitted because Safari's
    /// private (incognito) mode cannot be enabled purely via command-line and
    /// requires AppleScript or other automation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pathway::{ProfileOptions, ProfileType, WindowOptions};
    ///
    /// // Example: generate Safari profile arguments
    /// // let profile_opts = ProfileOptions {
    /// //     profile_type: ProfileType::Default,
    /// //     custom_args: Vec::new(),
    /// // };
    /// // let window_opts = WindowOptions::default();
    /// // let args = safari_profile_args(&profile_opts, &window_opts);
    /// // assert!(args.is_empty());
    /// ```
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

    /// Build command-line arguments for generic (non-browser-specific) window options.
    ///
    /// Currently only maps `incognito` to the common `--private` flag.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pathway::WindowOptions;
    ///
    /// // Example: generate generic window arguments
    /// // let opts = WindowOptions { new_window: false, incognito: true, kiosk: false };
    /// // let args = generic_window_args(&opts);
    /// // assert_eq!(args, vec!["--private".to_string()]);
    /// ```
    fn generic_window_args(window_opts: &WindowOptions) -> Vec<String> {
        let mut args = Vec::new();

        if window_opts.incognito {
            args.push("--private".to_string());
        }

        args
    }
}

/// Generate a hex-encoded, nanosecond-resolution timestamp string.
///
/// The returned string is the current system time since the UNIX epoch, encoded as lowercase hexadecimal
/// from the nanosecond count. Intended for use as a lightweight, mostly-unique identifier (e.g., temp
/// directory names).
///
/// # Examples
///
/// ```
/// // Example of what the function returns:
/// let example_id = "1a2b3c4d5e6f7890";
/// let value = u128::from_str_radix(example_id, 16).unwrap();
/// assert!(value > 0);
/// ```
fn generate_timestamp_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", timestamp)
}

/// Validate profile and window option combinations for a given browser and return any warnings.
///
/// This function checks for option conflicts and unsupported combinations and returns a list
/// of human-readable warning messages (empty if none). Examples of checked conditions:
/// - Using incognito with a non-default profile (incognito will ignore profile selection).
/// - Browser-specific unsupported profile types (e.g., Safari does not support named or custom directories).
/// - Browser-specific window option limitations (e.g., Safari kiosk/incognito not supported via CLI).
/// - Tor Browser and unknown browsers receive warnings about potential anonymity or compatibility issues.
///
/// # Returns
///
/// A `Result` containing a `Vec<String>` of warnings. The function does not return an error in
/// normal validation flows; `ProfileError` is reserved for underlying errors in other APIs.
///
/// # Examples
///
/// ```text
/// // Assume `browser`, `profile_opts`, and `window_opts` are constructed appropriately:
/// // let warnings = validate_profile_options(&browser, &profile_opts, &window_opts).unwrap();
/// // assert!(warnings.is_empty() || warnings.iter().any(|w| w.contains("does not support")));
/// ```
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
