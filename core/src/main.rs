use clap::{Parser, ValueEnum};
use pathway::{
    available_tokens, detect_inventory, find_browser, launch_with_profile, logging,
    validate_profile_options, validate_url, BrowserChannel, BrowserInfo, BrowserInventory,
    LaunchCommand, LaunchTarget, ProfileInfo, ProfileManager, ProfileOptions, ProfileType,
    SystemDefaultBrowser, ValidatedUrl, ValidationStatus, WindowOptions,
};
use serde::Serialize;
use std::path::PathBuf;
use std::process;
use tracing::{error, info, warn};

#[derive(Parser, Debug)]
#[command(author, version, about = "URL routing agent for Pathway", long_about = None)]
struct Args {
    /// Enable debug logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Output format
    #[arg(long, value_enum, default_value = "human", global = true)]
    format: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Open URLs in browsers
    Launch {
        /// URLs to open
        urls: Vec<String>,

        /// Browser to use (chrome, firefox, safari, etc.)
        #[arg(short, long)]
        browser: Option<String>,

        /// Browser channel (stable, beta, dev, canary, nightly)
        #[arg(long, value_enum)]
        channel: Option<BrowserChannelArg>,

        /// Use system default browser
        #[arg(long)]
        system_default: bool,

        /// Profile options (mutually exclusive)
        #[command(flatten)]
        profile: ProfileArgs,

        /// Window options
        #[command(flatten)]
        window: WindowArgs,

        /// Validate URLs but don't launch
        #[arg(long, alias = "dry-run")]
        no_launch: bool,
    },

    /// Manage browsers
    Browser {
        #[command(subcommand)]
        action: BrowserAction,
    },

    /// Manage browser profiles
    Profile {
        /// Browser to manage profiles for
        #[arg(short, long)]
        browser: Option<String>,

        /// Browser channel
        #[arg(long, value_enum)]
        channel: Option<BrowserChannelArg>,

        /// Custom user data directory to examine
        #[arg(long)]
        user_dir: Option<PathBuf>,

        #[command(subcommand)]
        action: ProfileAction,
    },
}

#[derive(Parser, Debug)]
enum BrowserAction {
    /// List all detected browsers
    List,
    /// Check if a specific browser is available
    Check {
        /// Browser name to check
        browser: String,
        /// Specific channel to check
        #[arg(long, value_enum)]
        channel: Option<BrowserChannelArg>,
    },
}

#[derive(Parser, Debug)]
enum ProfileAction {
    /// List available profiles
    List,
    /// Show detailed information about a profile
    Info {
        /// Profile name to show info for
        name: String,
    },
}

#[derive(Parser, Debug)]
#[group(required = false, multiple = false)]
struct ProfileArgs {
    /// Use specific browser profile
    #[arg(long)]
    profile: Option<String>,

    /// Use custom user data directory
    #[arg(long)]
    user_dir: Option<PathBuf>,

    /// Create temporary profile (deleted on exit)
    #[arg(long)]
    temp_profile: bool,

    /// Use guest profile (Chromium only)
    #[arg(long)]
    guest: bool,
}

#[derive(Parser, Debug)]
struct WindowArgs {
    /// Force new browser window
    #[arg(long)]
    new_window: bool,

    /// Open in incognito/private mode
    #[arg(long)]
    incognito: bool,

    /// Kiosk mode (fullscreen, no UI)
    #[arg(long)]
    kiosk: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum BrowserChannelArg {
    Stable,
    Beta,
    Dev,
    Canary,
    Nightly,
}

impl From<BrowserChannelArg> for BrowserChannel {
    fn from(value: BrowserChannelArg) -> Self {
        match value {
            BrowserChannelArg::Stable => BrowserChannel::Stable,
            BrowserChannelArg::Beta => BrowserChannel::Beta,
            BrowserChannelArg::Dev => BrowserChannel::Dev,
            BrowserChannelArg::Canary => BrowserChannel::Canary,
            BrowserChannelArg::Nightly => BrowserChannel::Nightly,
        }
    }
}

#[derive(Debug, Serialize)]
struct BrowserJson {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bundle_id: Option<String>,
    is_default: bool,
}

#[derive(Debug, Serialize)]
struct LaunchJsonResponse {
    action: &'static str,
    status: &'static str,
    urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    validated: Vec<ValidatedUrl>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warnings: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    browser: Option<BrowserJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    profile: Option<ProfileJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    window_options: Option<WindowOptionsJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<LaunchCommand>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct ListJsonResponse {
    action: &'static str,
    browsers: Vec<BrowserInfo>,
    system_default: SystemDefaultBrowser,
}

#[derive(Debug, Serialize)]
struct CheckJsonResponse {
    action: &'static str,
    browser: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel: Option<BrowserChannel>,
    available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolved: Option<BrowserInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct ProfileJson {
    #[serde(rename = "type")]
    profile_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

#[derive(Debug, Serialize)]
struct WindowOptionsJson {
    new_window: bool,
    incognito: bool,
    kiosk: bool,
}

#[derive(Debug, Serialize)]
struct ListProfilesResponse {
    action: &'static str,
    browser: String,
    profiles: Vec<ProfileInfo>,
}

#[derive(Debug, Serialize)]
struct ProfileInfoResponse {
    action: &'static str,
    browser: String,
    profile: ProfileInfo,
}

struct LaunchCommandParams {
    urls: Vec<String>,
    browser: Option<String>,
    channel: Option<BrowserChannelArg>,
    system_default: bool,
    profile_args: ProfileArgs,
    window_args: WindowArgs,
    no_launch: bool,
    format: OutputFormat,
}

fn main() {
    let args = Args::parse();

    if args.format == OutputFormat::Human {
        logging::setup_logging(args.verbose, false);
    }

    let inventory = detect_inventory();

    match args.command {
        Commands::Launch {
            urls,
            browser,
            channel,
            system_default,
            profile,
            window,
            no_launch,
        } => {
            let params = LaunchCommandParams {
                urls,
                browser,
                channel,
                system_default,
                profile_args: profile,
                window_args: window,
                no_launch,
                format: args.format,
            };
            handle_launch_command(&inventory, params);
        }
        Commands::Browser { action } => {
            handle_browser_command(&inventory, action, args.format);
        }
        Commands::Profile {
            browser,
            channel,
            user_dir,
            action,
        } => {
            handle_profile_command(&inventory, browser, channel, user_dir, action, args.format);
        }
    }
}

fn validate_urls(urls: &[String], format: OutputFormat) -> (Vec<ValidatedUrl>, bool) {
    let mut results = Vec::new();
    let mut has_error = false;

    for (index, url) in urls.iter().enumerate() {
        match validate_url(url) {
            Ok(validated) => {
                if format == OutputFormat::Human {
                    if let Some(warning) = &validated.warning {
                        info!(
                            "URL {}: {} (scheme: {}) - WARNING: {}",
                            index + 1,
                            validated.normalized,
                            validated.scheme,
                            warning
                        );
                    } else {
                        info!(
                            "URL validated: {} (scheme: {})",
                            validated.normalized, validated.scheme
                        );
                    }
                }
                results.push(validated);
            }
            Err(e) => {
                has_error = true;
                let invalid = ValidatedUrl {
                    original: url.clone(),
                    url: url.clone(),
                    normalized: url.clone(),
                    scheme: String::new(),
                    status: ValidationStatus::Invalid,
                    warning: Some(e.to_string()),
                };
                results.push(invalid);

                if format == OutputFormat::Human {
                    error!("URL {}: {}", index + 1, e);
                }
            }
        }
    }

    (results, has_error)
}

fn select_browser<'a>(
    inventory: &'a BrowserInventory,
    browser: Option<&str>,
    channel: Option<BrowserChannel>,
    system_default: bool,
) -> Option<&'a BrowserInfo> {
    if system_default {
        None
    } else if let Some(name) = browser {
        find_browser(&inventory.browsers, name, channel)
    } else {
        None
    }
}

fn validate_and_prepare_options(
    browser: Option<&BrowserInfo>,
    profile_args: &ProfileArgs,
    window_args: &WindowArgs,
    format: OutputFormat,
) -> (ProfileOptions, WindowOptions, Vec<String>) {
    let mut warnings = Vec::new();
    let profile_options = convert_profile_args(profile_args, &mut warnings);
    let window_options = convert_window_args(window_args);

    if let Some(browser) = browser {
        match validate_profile_options(browser, &profile_options, &window_options) {
            Ok(profile_warnings) => {
                if format == OutputFormat::Human {
                    for warning in &profile_warnings {
                        warn!("{}", warning);
                    }
                }
                warnings.extend(profile_warnings);
            }
            Err(e) => {
                if format == OutputFormat::Human {
                    error!("Profile validation error: {}", e);
                } else {
                    warnings.push(format!("Profile validation error: {}", e));
                }
            }
        }
    } else {
        // Validate system default limitations
        let has_profile_options = !matches!(profile_options.profile_type, ProfileType::Default);
        let has_window_options =
            window_options.new_window || window_options.incognito || window_options.kiosk;

        if has_profile_options {
            let warning = "Profile options require specifying a browser with --browser".to_string();
            if format == OutputFormat::Human {
                warn!("{}", warning);
            }
            warnings.push(warning);
        }

        if has_window_options {
            let warning = "Window options require specifying a browser with --browser".to_string();
            if format == OutputFormat::Human {
                warn!("{}", warning);
            }
            warnings.push(warning);
        }
    }

    (profile_options, window_options, warnings)
}

fn handle_launch_command(inventory: &BrowserInventory, params: LaunchCommandParams) {
    let LaunchCommandParams {
        urls,
        browser,
        channel,
        system_default,
        profile_args,
        window_args,
        no_launch,
        format,
    } = params;

    let (results, has_error) = validate_urls(&urls, format);
    let normalized_urls: Vec<String> = results.iter().map(|url| url.normalized.clone()).collect();

    if has_error {
        if format == OutputFormat::Json {
            let response = LaunchJsonResponse {
                action: "launch",
                status: "error",
                urls: normalized_urls.clone(),
                url: normalized_urls.first().cloned(),
                validated: results.clone(),
                warnings: None,
                browser: None,
                profile: None,
                window_options: None,
                command: None,
                message: Some("URL validation failed".to_string()),
            };
            println!("{}", serde_json::to_string_pretty(&response).unwrap());
        }
        process::exit(1);
    }

    let requested_channel = channel.map(Into::into);
    let selected_browser = select_browser(
        inventory,
        browser.as_deref(),
        requested_channel,
        system_default,
    );

    let mut additional_warnings = Vec::new();
    if browser.is_some() && selected_browser.is_none() {
        let mut warning = format!("Browser '{}' not found", browser.as_deref().unwrap());
        if let Some(channel) = requested_channel {
            warning.push_str(&format!(" (channel: {})", channel.canonical_name()));
        }
        warning.push_str(&format!(
            ". Available browsers: {}",
            available_tokens(&inventory.browsers).join(", ")
        ));

        if format == OutputFormat::Human {
            warn!("{}", warning);
        }
        additional_warnings.push(warning);
    }

    let (profile_options, window_options, mut warnings) =
        validate_and_prepare_options(selected_browser, &profile_args, &window_args, format);

    warnings.extend(additional_warnings);

    let launch_target = selected_browser
        .map(LaunchTarget::Browser)
        .unwrap_or(LaunchTarget::SystemDefault);

    if no_launch {
        if format == OutputFormat::Human {
            if let Some(browser) = selected_browser {
                let profile_info = get_profile_description(&profile_options);
                info!(
                    "Launch skipped (--no-launch). Would launch in {}{}",
                    browser.display_name.as_str(),
                    profile_info
                );
            } else {
                info!(
                    "Launch skipped (--no-launch). Would launch in {}",
                    inventory.system_default.display_name.as_str()
                );
            }
        } else {
            let browser_json = selected_browser
                .map(|info| BrowserJson::from_browser(info, false))
                .unwrap_or_else(|| BrowserJson::from_system_default(&inventory.system_default));

            let response = LaunchJsonResponse {
                action: "launch",
                status: "skipped",
                urls: normalized_urls.clone(),
                url: normalized_urls.first().cloned(),
                validated: results.clone(),
                warnings: if warnings.is_empty() {
                    None
                } else {
                    Some(warnings.clone())
                },
                browser: Some(browser_json),
                profile: Some(ProfileJson::from_profile_options(&profile_options)),
                window_options: Some(WindowOptionsJson::from_window_options(&window_options)),
                command: None,
                message: Some("Launch skipped (--no-launch)".to_string()),
            };
            println!("{}", serde_json::to_string_pretty(&response).unwrap());
        }
        return;
    }

    let (profile_opts, window_opts) = if selected_browser.is_some() {
        (Some(&profile_options), Some(&window_options))
    } else {
        (None, None)
    };

    match launch_with_profile(launch_target, &normalized_urls, profile_opts, window_opts) {
        Ok(outcome) => {
            if format == OutputFormat::Human {
                if let Some(browser) = selected_browser {
                    let profile_info = get_profile_description(&profile_options);
                    info!(
                        "Launching in {}{}: {}",
                        browser.display_name,
                        profile_info,
                        normalized_urls.join(", ")
                    );
                } else {
                    let name = outcome
                        .system_default
                        .as_ref()
                        .map(|b| b.display_name.as_str())
                        .unwrap_or("system default browser");
                    info!("Launching in {}: {}", name, normalized_urls.join(", "));
                }
            } else {
                let browser_json = outcome
                    .browser
                    .as_ref()
                    .map(|info| BrowserJson::from_browser(info, false))
                    .or_else(|| {
                        outcome
                            .system_default
                            .as_ref()
                            .map(BrowserJson::from_system_default)
                    });

                let response = LaunchJsonResponse {
                    action: "launch",
                    status: "success",
                    urls: normalized_urls.clone(),
                    url: normalized_urls.first().cloned(),
                    validated: results.clone(),
                    warnings: if warnings.is_empty() {
                        None
                    } else {
                        Some(warnings)
                    },
                    browser: browser_json,
                    profile: Some(ProfileJson::from_profile_options(&profile_options)),
                    window_options: Some(WindowOptionsJson::from_window_options(&window_options)),
                    command: Some(outcome.command.clone()),
                    message: None,
                };
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            }
        }
        Err(err) => {
            let message = format!("Failed to launch browser: {}", err);
            if format == OutputFormat::Human {
                error!("{}", message);
            } else {
                let browser_json = selected_browser
                    .map(|info| BrowserJson::from_browser(info, false))
                    .or_else(|| Some(BrowserJson::from_system_default(&inventory.system_default)));

                let response = LaunchJsonResponse {
                    action: "launch",
                    status: "error",
                    urls: normalized_urls.clone(),
                    url: normalized_urls.first().cloned(),
                    validated: results.clone(),
                    warnings: if warnings.is_empty() {
                        None
                    } else {
                        Some(warnings)
                    },
                    browser: browser_json,
                    profile: Some(ProfileJson::from_profile_options(&profile_options)),
                    window_options: Some(WindowOptionsJson::from_window_options(&window_options)),
                    command: None,
                    message: Some(message.clone()),
                };
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            }
            process::exit(1);
        }
    }
}

fn handle_browser_command(
    inventory: &BrowserInventory,
    action: BrowserAction,
    format: OutputFormat,
) {
    match action {
        BrowserAction::List => match format {
            OutputFormat::Human => {
                println!("Detected browsers:");
                if inventory.browsers.is_empty() {
                    println!("  (none)");
                } else {
                    for browser in &inventory.browsers {
                        let path = browser
                            .bundle_path
                            .as_ref()
                            .or(browser.executable.as_ref())
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "(unknown path)".to_string());

                        if let Some(bundle_id) = &browser.bundle_id {
                            println!(
                                "  {} ({}) - {} [{}]",
                                browser.cli_name,
                                browser.channel.canonical_name(),
                                path,
                                bundle_id
                            );
                        } else {
                            println!(
                                "  {} ({}) - {}",
                                browser.cli_name,
                                browser.channel.canonical_name(),
                                path
                            );
                        }
                    }
                }
                println!("System default: {}", inventory.system_default.display_name);
            }
            OutputFormat::Json => {
                let response = ListJsonResponse {
                    action: "list-browsers",
                    browsers: inventory.browsers.clone(),
                    system_default: inventory.system_default.clone(),
                };
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            }
        },
        BrowserAction::Check { browser, channel } => {
            let requested_channel = channel.map(Into::into);
            let result = find_browser(&inventory.browsers, &browser, requested_channel);

            match format {
                OutputFormat::Human => {
                    if let Some(info) = result {
                        let path = info
                            .bundle_path
                            .as_ref()
                            .or(info.executable.as_ref())
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "(unknown path)".to_string());

                        if let Some(bundle_id) = &info.bundle_id {
                            println!(
                                "Browser '{}' ({}) is available at {} [{}]",
                                info.cli_name,
                                info.channel.canonical_name(),
                                path,
                                bundle_id
                            );
                        } else {
                            println!(
                                "Browser '{}' ({}) is available at {}",
                                info.cli_name,
                                info.channel.canonical_name(),
                                path
                            );
                        }
                    } else {
                        println!(
                            "Browser '{}' not found. Available browsers: {}",
                            browser,
                            available_tokens(&inventory.browsers).join(", ")
                        );
                        process::exit(1);
                    }
                }
                OutputFormat::Json => {
                    let response = CheckJsonResponse {
                        action: "check-browser",
                        browser: browser.to_string(),
                        channel: requested_channel,
                        available: result.is_some(),
                        resolved: result.cloned(),
                        message: if result.is_none() {
                            Some(format!(
                                "Browser '{}' not found. Available browsers: {}",
                                browser,
                                available_tokens(&inventory.browsers).join(", ")
                            ))
                        } else {
                            None
                        },
                    };
                    println!("{}", serde_json::to_string_pretty(&response).unwrap());
                    if result.is_none() {
                        process::exit(1);
                    }
                }
            }
        }
    }
}

fn handle_profile_command(
    inventory: &BrowserInventory,
    browser: Option<String>,
    channel: Option<BrowserChannelArg>,
    user_dir: Option<PathBuf>,
    action: ProfileAction,
    format: OutputFormat,
) {
    let browser_name = browser.as_deref().unwrap_or("chrome");
    let requested_channel = channel.map(Into::into);

    let browser = match find_browser(&inventory.browsers, browser_name, requested_channel) {
        Some(info) => info,
        None => {
            let error_msg = format!(
                "Browser '{}' not found. Available browsers: {}",
                browser_name,
                available_tokens(&inventory.browsers).join(", ")
            );

            if format == OutputFormat::Human {
                error!("{}", error_msg);
            }
            process::exit(1);
        }
    };

    let custom_dir = user_dir.as_deref();

    match action {
        ProfileAction::List => {
            match ProfileManager::discover_profiles_in_directory(browser, custom_dir) {
                Ok(profiles) => {
                    if format == OutputFormat::Human {
                        println!("{} profiles:", browser.display_name);
                        if profiles.is_empty() {
                            println!("  (none)");
                        } else {
                            for profile in &profiles {
                                let default_marker =
                                    if profile.is_default { " (default)" } else { "" };
                                let last_used = profile
                                    .last_used
                                    .as_ref()
                                    .map(|t| format!(" - Last used: {}", t))
                                    .unwrap_or_default();

                                // Show directory name if different from display name
                                let dir_info = if profile.name != profile.display_name
                                    && !profile.path.as_os_str().is_empty()
                                {
                                    format!(" [{}]", profile.name)
                                } else {
                                    String::new()
                                };

                                println!(
                                    "  {}{}{}{}",
                                    profile.display_name, dir_info, default_marker, last_used
                                );
                            }
                        }
                    } else {
                        let response = ListProfilesResponse {
                            action: "list-profiles",
                            browser: browser.display_name.clone(),
                            profiles,
                        };
                        println!("{}", serde_json::to_string_pretty(&response).unwrap());
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to discover profiles: {}", e);
                    if format == OutputFormat::Human {
                        error!("{}", error_msg);
                    }
                    process::exit(1);
                }
            }
        }
        ProfileAction::Info { name } => {
            match ProfileManager::find_profile_in_directory(browser, &name, custom_dir) {
                Ok(profile) => {
                    if format == OutputFormat::Human {
                        println!("Profile: {}", profile.display_name);
                        println!("  Name: {}", profile.name);
                        println!("  Path: {}", profile.path.display());
                        println!(
                            "  Default: {}",
                            if profile.is_default { "Yes" } else { "No" }
                        );
                        if let Some(last_used) = &profile.last_used {
                            println!("  Last used: {}", last_used);
                        }
                        println!("  Browser: {}", browser.display_name);
                    } else {
                        let response = ProfileInfoResponse {
                            action: "profile-info",
                            browser: browser.display_name.clone(),
                            profile,
                        };
                        println!("{}", serde_json::to_string_pretty(&response).unwrap());
                    }
                }
                Err(e) => {
                    let error_msg = format!("Profile '{}' not found: {}", name, e);
                    if format == OutputFormat::Human {
                        error!("{}", error_msg);
                    }
                    process::exit(1);
                }
            }
        }
    }
}

fn convert_profile_args(profile_args: &ProfileArgs, warnings: &mut Vec<String>) -> ProfileOptions {
    let profile_type = if profile_args.temp_profile {
        match ProfileManager::create_temp_profile() {
            Ok(temp_path) => {
                info!(
                    "Created temporary profile directory: {}",
                    temp_path.display()
                );
                ProfileType::Temporary(temp_path)
            }
            Err(e) => {
                error!("Failed to create temporary profile: {}", e);
                warnings.push(format!(
                    "Failed to create temporary profile: {}. Please check disk space and permissions.",
                    e
                ));
                ProfileType::Default
            }
        }
    } else if let Some(user_dir) = &profile_args.user_dir {
        match ProfileManager::prepare_custom_directory(user_dir) {
            Ok(prepared_path) => ProfileType::CustomDirectory(prepared_path),
            Err(e) => {
                warnings.push(format!("Failed to prepare custom directory: {}", e));
                ProfileType::Default
            }
        }
    } else if profile_args.guest {
        ProfileType::Guest
    } else if let Some(profile_name) = &profile_args.profile {
        ProfileType::Named(profile_name.clone())
    } else {
        ProfileType::Default
    };

    ProfileOptions {
        profile_type,
        custom_args: Vec::new(),
    }
}

fn convert_window_args(window_args: &WindowArgs) -> WindowOptions {
    WindowOptions {
        new_window: window_args.new_window,
        incognito: window_args.incognito,
        kiosk: window_args.kiosk,
    }
}

// Helper implementations
impl BrowserJson {
    fn from_browser(info: &BrowserInfo, is_default: bool) -> Self {
        BrowserJson {
            name: info.cli_name.clone(),
            channel: Some(info.channel.canonical_name().to_string()),
            path: info
                .bundle_path
                .as_ref()
                .or(info.executable.as_ref())
                .map(|p| p.display().to_string()),
            bundle_id: info.bundle_id.clone(),
            is_default,
        }
    }

    fn from_system_default(default: &SystemDefaultBrowser) -> Self {
        BrowserJson {
            name: default.display_name.clone(),
            channel: default
                .channel
                .map(|channel| channel.canonical_name().to_string()),
            path: default.path.as_ref().map(|p| p.display().to_string()),
            bundle_id: None,
            is_default: true,
        }
    }
}

impl ProfileJson {
    fn from_profile_options(profile_opts: &ProfileOptions) -> Self {
        match &profile_opts.profile_type {
            ProfileType::Default => ProfileJson {
                profile_type: "default".to_string(),
                name: None,
                path: None,
            },
            ProfileType::Named(name) => ProfileJson {
                profile_type: "named".to_string(),
                name: Some(name.clone()),
                path: None,
            },
            ProfileType::CustomDirectory(path) => ProfileJson {
                profile_type: "custom".to_string(),
                name: None,
                path: Some(path.display().to_string()),
            },
            ProfileType::Temporary(path) => ProfileJson {
                profile_type: "temporary".to_string(),
                name: None,
                path: Some(path.display().to_string()),
            },
            ProfileType::Guest => ProfileJson {
                profile_type: "guest".to_string(),
                name: None,
                path: None,
            },
        }
    }
}

impl WindowOptionsJson {
    fn from_window_options(window_opts: &WindowOptions) -> Self {
        WindowOptionsJson {
            new_window: window_opts.new_window,
            incognito: window_opts.incognito,
            kiosk: window_opts.kiosk,
        }
    }
}

fn get_profile_description(profile_opts: &ProfileOptions) -> String {
    match &profile_opts.profile_type {
        ProfileType::Default => String::new(),
        ProfileType::Named(name) => format!(" with profile '{}'", name),
        ProfileType::CustomDirectory(path) => {
            format!(" with custom directory '{}'", path.display())
        }
        ProfileType::Temporary(path) => format!(" with temporary profile ({})", path.display()),
        ProfileType::Guest => " in guest mode".to_string(),
    }
}
