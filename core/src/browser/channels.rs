use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
pub enum ChromiumChannel {
    Stable,
    Beta,
    Dev,
    Canary,
}

impl ChromiumChannel {
    pub fn canonical_name(self) -> &'static str {
        match self {
            ChromiumChannel::Stable => "stable",
            ChromiumChannel::Beta => "beta",
            ChromiumChannel::Dev => "dev",
            ChromiumChannel::Canary => "canary",
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
pub enum FirefoxChannel {
    Stable,
    Beta,
    Dev,
    Nightly,
    Esr,
}

impl FirefoxChannel {
    pub fn canonical_name(self) -> &'static str {
        match self {
            FirefoxChannel::Stable => "stable",
            FirefoxChannel::Beta => "beta",
            FirefoxChannel::Dev => "dev",
            FirefoxChannel::Nightly => "nightly",
            FirefoxChannel::Esr => "esr",
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
pub enum SafariChannel {
    Stable,
    TechnologyPreview,
}

impl SafariChannel {
    pub fn canonical_name(self) -> &'static str {
        match self {
            SafariChannel::Stable => "stable",
            SafariChannel::TechnologyPreview => "technology-preview",
        }
    }
}

// General enum to hold the specific channel type
#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
pub enum BrowserChannel {
    Chromium(ChromiumChannel),
    Firefox(FirefoxChannel),
    Safari(SafariChannel),
    /// Represents browsers that have a single release channel
    Single,
}

impl BrowserChannel {
    pub fn canonical_name(self) -> &'static str {
        match self {
            BrowserChannel::Chromium(c) => c.canonical_name(),
            BrowserChannel::Firefox(c) => c.canonical_name(),
            BrowserChannel::Safari(c) => c.canonical_name(),
            BrowserChannel::Single => "stable",
        }
    }
}
