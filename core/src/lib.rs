pub mod browser;
pub mod error;
pub mod logging;
pub mod profile;
pub mod url;

pub use browser::{
    available_tokens, detect_inventory, find_browser, launch, launch_with_profile, BrowserChannel,
    BrowserInfo, BrowserInventory, BrowserKind, LaunchCommand, LaunchError, LaunchOutcome,
    LaunchTarget, SystemDefaultBrowser,
};
pub use error::{PathwayError, Result};
pub use profile::{
    validate_profile_options, ProfileInfo, ProfileManager, ProfileOptions, ProfileType,
    WindowOptions,
};
pub use url::{validate_url, ValidatedUrl, ValidationStatus};
