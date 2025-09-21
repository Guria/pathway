use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
pub enum MacOsInstallationSource {
    System,
    Homebrew,
    AppStore,
}

impl MacOsInstallationSource {
    pub fn canonical_name(self) -> &'static str {
        match self {
            MacOsInstallationSource::System => "system",
            MacOsInstallationSource::Homebrew => "homebrew",
            MacOsInstallationSource::AppStore => "app-store",
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
pub enum LinuxInstallationSource {
    System,
    Flatpak,
    Snap,
}

impl LinuxInstallationSource {
    pub fn canonical_name(self) -> &'static str {
        match self {
            LinuxInstallationSource::System => "system",
            LinuxInstallationSource::Flatpak => "flatpak",
            LinuxInstallationSource::Snap => "snap",
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
pub enum WindowsInstallationSource {
    System,
    Chocolatey,
    Scoop,
    Winget,
}

impl WindowsInstallationSource {
    pub fn canonical_name(self) -> &'static str {
        match self {
            WindowsInstallationSource::System => "system",
            WindowsInstallationSource::Chocolatey => "chocolatey",
            WindowsInstallationSource::Scoop => "scoop",
            WindowsInstallationSource::Winget => "winget",
        }
    }
}

// General enum to hold the platform-specific source
#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
pub enum InstallationSource {
    MacOs(MacOsInstallationSource),
    Linux(LinuxInstallationSource),
    Windows(WindowsInstallationSource),
    Unknown,
}

impl InstallationSource {
    pub fn canonical_name(self) -> &'static str {
        match self {
            InstallationSource::MacOs(s) => s.canonical_name(),
            InstallationSource::Linux(s) => s.canonical_name(),
            InstallationSource::Windows(s) => s.canonical_name(),
            InstallationSource::Unknown => "unknown",
        }
    }
}

/// On-demand installation source detection for when it's actually needed
pub fn detect_installation_source(browser: &crate::browser::BrowserInfo) -> InstallationSource {
    #[cfg(target_os = "macos")]
    {
        crate::browser::macos::detect_source_for_browser(browser)
    }
    #[cfg(target_os = "linux")]
    {
        crate::browser::linux::detect_source_for_browser(browser)
    }
    #[cfg(target_os = "windows")]
    {
        crate::browser::windows::detect_source_for_browser(browser)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        InstallationSource::Unknown
    }
}
