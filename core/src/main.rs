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
        #[arg(long, conflicts_with_all = ["browser", "channel"])]
        system_default: bool,

        /// Force fallback browser when no --browser is provided (prevents infinite loops when launched from app bundle)
        #[arg(long, conflicts_with_all = ["system_default", "browser", "channel"])]
        no_system_default: bool,

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
    #[arg(long, conflicts_with_all = ["temp_profile", "guest"])]
    profile: Option<String>,

    /// Use custom user data directory
    #[arg(long, conflicts_with_all = ["temp_profile", "guest"])]
    user_dir: Option<PathBuf>,

    /// Create temporary profile (deleted on exit)
    #[arg(long, conflicts_with_all = ["profile", "user_dir", "guest"])]
    temp_profile: bool,

    /// Use guest profile (Chromium only)
    #[arg(long, conflicts_with_all = ["profile", "user_dir", "temp_profile"])]
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

#[derive(Debug, Serialize)]
struct ProfileErrorResponse {
    action: &'static str,
    browser: String,
    message: String,
}

struct LaunchCommandParams {
    urls: Vec<String>,
    browser: Option<String>,
    channel: Option<BrowserChannelArg>,
    system_default: bool,
    no_system_default: bool,
    profile_args: ProfileArgs,
    window_args: WindowArgs,
    no_launch: bool,
    format: OutputFormat,
}

/// Get a safe fallback browser when infinite loop prevention is needed.
/// Uses OS-appropriate browser preferences for reliability.
fn get_fallback_browser(inventory: &BrowserInventory) -> Option<&BrowserInfo> {
    // OS-specific fallback preferences
    let fallback_preferences = if cfg!(target_os = "macos") {
        &["safari", "chrome", "firefox"][..]
    } else if cfg!(target_os = "windows") {
        &["edge", "chrome", "firefox"][..]
    } else {
        // Linux and other platforms
        &["chrome", "firefox", "chromium"][..]
    };

    // Try each preferred browser in order
    for browser_name in fallback_preferences {
        if let Some(browser) = find_browser(&inventory.browsers, browser_name, None) {
            return Some(browser);
        }
    }

    // Fallback to first available browser if preferred ones not found
    inventory.browsers.first()
}

/// Entry point for the CLI executable.
///
/// Parses command-line arguments, sets up human-mode logging when requested,
/// detects available browsers, and dispatches to the selected subcommand:
/// Launch, Browser, or Profile. Each subcommand handles validation, JSON or
/// human output, and may exit the process on fatal errors.
///
/// This function does not return a value and drives the program's top-level
/// control flow (argument parsing → inventory detection → command dispatch).
///
/// # Examples
///
/// ```no_run
/// // Example invocations (run from a shell):
/// // Launch a URL with the system default browser:
/// //   pathway-agent launch https://example.com --system-default
///
/// // List detected browsers in JSON:
/// //   pathway-agent browser list --format json
///
/// // Show profile info for a named browser:
/// //   pathway-agent profile --browser chrome info "Default"
/// ```
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
            no_system_default,
            profile,
            window,
            no_launch,
        } => {
            let params = LaunchCommandParams {
                urls,
                browser,
                channel,
                system_default,
                no_system_default,
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

/// Validate a list of URL strings and return per-URL validation results plus a flag indicating
/// whether any URL failed validation.
///
/// On success each entry is a `ValidatedUrl` (may include a non-fatal `warning`). On failure the
/// corresponding `ValidatedUrl` will have `status == ValidationStatus::Invalid` and its `warning`
/// will contain the validation error message. When `format == OutputFormat::Human` the function
/// emits informational or error messages for each URL.
///
/// # Returns
///
/// A tuple `(Vec<ValidatedUrl>, bool)` where the boolean is `true` if any URL failed validation.
///
/// # Examples
///
/// ```
/// let urls = vec![
///     "https://example.com".to_string(),
///     "not-a-url".to_string(),
/// ];
/// let (results, has_error) = validate_urls(&urls, OutputFormat::Json);
/// assert_eq!(results.len(), 2);
/// assert!(has_error);
/// assert_eq!(results[0].status, ValidationStatus::Valid);
/// assert_eq!(results[1].status, ValidationStatus::Invalid);
/// ```
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

/// Choose a BrowserInfo from the inventory unless the system default is requested.
///
/// Returns:
/// - None when `system_default` is true or when no browser name is provided.
/// - The result of `find_browser` when a browser `name` is given (which may be `Some(&BrowserInfo)` or `None` if not found).
///
/// The `channel` argument is forwarded to the browser lookup when a `name` is supplied.
///
/// # Examples
///
/// ```no_run
/// // Returns None because system default was requested
/// let chosen = select_browser(&inventory, None, None, true);
/// assert!(chosen.is_none());
///
/// // When a browser name is provided, the lookup result (Some or None) is returned
/// let chosen = select_browser(&inventory, Some("firefox"), Some(BrowserChannel::Stable), false);
/// ```
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

/// Validate profile/window CLI arguments and convert them to runtime options.
///
/// Converts `ProfileArgs` and `WindowArgs` into `ProfileOptions` and `WindowOptions`,
/// runs platform-specific validation when a concrete `browser` is provided, and
/// collects any warnings produced during conversion or validation.
///
/// Behavior:
/// - If `browser` is `Some`, calls `validate_profile_options(browser, &profile_options, &window_options)`.
///   Any warnings from that validation are appended to the returned warnings. Validation errors
///   are logged with `error!` in Human format; in non-human formats the error message is added
///   to the returned warnings.
/// - If `browser` is `None` (system-default mode), profile- or window-related options that
///   require an explicit browser are not validated; instead a warning is produced for each such
///   option indicating that `--browser` is required. Warnings are also logged with `warn!` in
///   Human format.
///
/// Returns a tuple `(ProfileOptions, WindowOptions, Vec<String>)` where the vector contains
/// accumulated warning/error messages (strings). The function does not return a `Result` and
/// never panics on validation failures — those are reported through logging and the warnings vector.
///
/// # Examples
///
/// ```
/// // Assume types and helpers are in scope for this crate.
/// let profile_args = ProfileArgs::default();
/// let window_args = WindowArgs::default();
/// let (profile_opts, window_opts, warnings) = validate_and_prepare_options(
///     None, // use system default browser
///     &profile_args,
///     &window_args,
///     OutputFormat::Human,
/// );
/// assert!(warnings.is_empty() || warnings.iter().all(|w| w.contains("--browser") || !w.is_empty()));
/// ```
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

/// Handle the "launch" subcommand: validate URLs, resolve the target browser (or system default),
/// prepare profile/window options, and perform (or skip) the launch. Outputs either human-readable
/// logs or structured JSON depending on `params.format`.
///
/// On URL validation or launch failure this function will print a JSON error (when JSON mode) or log
/// an error (when human mode) and terminate the process with exit code 1. In no-launch/dry-run
/// mode it prints a skipped message (or JSON response) and returns without launching. Successful
/// launches print a success message or a structured JSON response containing the resolved browser,
/// profile and window options, and the launcher command when available.
///
/// Side effects:
/// - Writes to stdout (JSON responses) or to the configured logging/tracing sink (human output).
/// - May call process::exit(1) on failures.
/// - May invoke the platform browser launch when not in no-launch mode.
///
/// # Parameters
///
/// - `inventory`: available browser inventory and system default used to resolve targets.
/// - `params`: aggregated parameters for the launch operation (URLs, browser/channel selection,
///   system-default flag, profile/window args, no-launch flag, and output format).
///
/// # Examples
///
/// ```
/// // Construct an inventory and params (omitted here) then call:
/// // handle_launch_command(&inventory, params);
/// ```
fn handle_launch_command(inventory: &BrowserInventory, params: LaunchCommandParams) {
    let LaunchCommandParams {
        urls,
        browser,
        channel,
        system_default,
        no_system_default,
        profile_args,
        window_args,
        no_launch,
        format,
    } = params;

    let (results, has_error) = validate_urls(&urls, format);
    let normalized_urls: Vec<String> = results.iter().map(|url| url.normalized.clone()).collect();

    if has_error {
        handle_url_validation_error(&normalized_urls, &results, format);
        process::exit(1);
    }

    let requested_channel = channel.map(Into::into);
    let mut selected_browser = select_browser(
        inventory,
        browser.as_deref(),
        requested_channel,
        system_default,
    );

    // Force fallback browser when --no-system-default is used
    let mut is_fallback = false;
    if no_system_default && selected_browser.is_none() {
        selected_browser = get_fallback_browser(inventory);
        is_fallback = true;

        if selected_browser.is_none() {
            let error_msg = "No fallback browser available";
            if format == OutputFormat::Human {
                error!("{}", error_msg);
            } else {
                print_launch_error_json(&normalized_urls, &results, error_msg);
            }
            process::exit(1);
        }
    }

    let additional_warnings = generate_browser_warnings(
        &browser,
        selected_browser,
        requested_channel,
        inventory,
        format,
        is_fallback,
    );

    let (profile_options, window_options, mut warnings) =
        validate_and_prepare_options(selected_browser, &profile_args, &window_args, format);

    warnings.extend(additional_warnings);

    let launch_target = if is_fallback {
        // Use the fallback browser directly instead of system default
        LaunchTarget::Browser(selected_browser.unwrap())
    } else {
        selected_browser
            .map(LaunchTarget::Browser)
            .unwrap_or(LaunchTarget::SystemDefault)
    };

    if no_launch {
        let response_data = LaunchResponseData {
            selected_browser,
            inventory,
            normalized_urls: &normalized_urls,
            results: &results,
            warnings: &warnings,
            format,
        };
        handle_no_launch_response(&profile_options, &window_options, response_data);
        return;
    }

    let response_data = LaunchResponseData {
        selected_browser,
        inventory,
        normalized_urls: &normalized_urls,
        results: &results,
        warnings: &warnings,
        format,
    };
    execute_launch_and_respond(
        launch_target,
        &profile_options,
        &window_options,
        response_data,
    );
}

/// Response data for browser launch operations
struct LaunchResponseData<'a> {
    selected_browser: Option<&'a BrowserInfo>,
    inventory: &'a BrowserInventory,
    normalized_urls: &'a [String],
    results: &'a [ValidatedUrl],
    warnings: &'a [String],
    format: OutputFormat,
}

/// Execute the browser launch and handle the response
fn execute_launch_and_respond(
    launch_target: LaunchTarget,
    profile_options: &ProfileOptions,
    window_options: &WindowOptions,
    response_data: LaunchResponseData,
) {
    let (profile_opts, window_opts) = if response_data.selected_browser.is_some() {
        (Some(profile_options), Some(window_options))
    } else {
        (None, None)
    };

    match launch_with_profile(
        launch_target,
        response_data.normalized_urls,
        profile_opts,
        window_opts,
    ) {
        Ok(outcome) => {
            if response_data.format == OutputFormat::Human {
                if let Some(browser) = response_data.selected_browser {
                    let profile_info = get_profile_description(profile_options);
                    info!(
                        "Launching in {}{}: {}",
                        browser.display_name,
                        profile_info,
                        response_data.normalized_urls.join(", ")
                    );
                } else {
                    let name = outcome
                        .system_default
                        .as_ref()
                        .map(|b| b.display_name.as_str())
                        .unwrap_or("system default browser");
                    info!(
                        "Launching in {}: {}",
                        name,
                        response_data.normalized_urls.join(", ")
                    );
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

                let response = build_launch_json_response(
                    "success",
                    response_data.normalized_urls,
                    response_data.results,
                    response_data.warnings,
                    browser_json,
                    response_data.selected_browser,
                    profile_options,
                    window_options,
                    Some(outcome.command.clone()),
                    None,
                );
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            }
        }
        Err(err) => {
            let message = format!("Failed to launch browser: {}", err);
            if response_data.format == OutputFormat::Human {
                error!("{}", message);
            } else {
                let browser_json = response_data
                    .selected_browser
                    .map(|info| BrowserJson::from_browser(info, false))
                    .or_else(|| {
                        Some(BrowserJson::from_system_default(
                            &response_data.inventory.system_default,
                        ))
                    });

                let response = build_launch_json_response(
                    "error",
                    response_data.normalized_urls,
                    response_data.results,
                    response_data.warnings,
                    browser_json,
                    response_data.selected_browser,
                    profile_options,
                    window_options,
                    None,
                    Some(message.clone()),
                );
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            }
            process::exit(1);
        }
    }
}

/// Handle the `browser` subcommand: list detected browsers or check availability of a specific browser.
///
/// - In `List` mode, prints either a human-readable list of detected browsers and the system default,
///   or emits a `ListJsonResponse` JSON object when `format` is `OutputFormat::Json`.
/// - In `Check` mode, looks up the named browser (optionally constrained to a channel) and reports its
///   availability in the selected `format`. When `OutputFormat::Human` it prints a message; when `OutputFormat::Json`
///   it emits a `CheckJsonResponse` JSON object.
///
/// Side effects:
/// - May call `std::process::exit(1)` if a `Check` request cannot find the requested browser (both in human and JSON modes).
///
/// Parameters:
/// - `inventory`: the detected browser inventory to query.
/// - `action`: the browser action to perform (`List` or `Check`).
/// - `format`: output format (`Human` or `Json`).
///
/// # Examples
///
/// ```
/// # use core::cli::{handle_browser_command, BrowserAction, OutputFormat};
/// # use pathway::BrowserInventory;
/// // Assume `inventory` is populated by detection logic.
/// // List browsers in human form:
/// // handle_browser_command(&inventory, BrowserAction::List, OutputFormat::Human);
/// // Check a browser and print JSON:
/// // handle_browser_command(&inventory, BrowserAction::Check { browser: "chrome".into(), channel: None }, OutputFormat::Json);
/// ```
fn handle_browser_command(
    inventory: &BrowserInventory,
    action: BrowserAction,
    format: OutputFormat,
) {
    match action {
        BrowserAction::List => match format {
            OutputFormat::Human => {
                eprintln!("Detected browsers:");
                if inventory.browsers.is_empty() {
                    eprintln!("  (none)");
                } else {
                    for browser in &inventory.browsers {
                        let path = browser
                            .bundle_path
                            .as_ref()
                            .or(browser.executable.as_ref())
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "(unknown path)".to_string());

                        if let Some(bundle_id) = &browser.bundle_id {
                            eprintln!(
                                "  {} ({}) - {} [{}]",
                                browser.cli_name,
                                browser.channel.canonical_name(),
                                path,
                                bundle_id
                            );
                        } else {
                            eprintln!(
                                "  {} ({}) - {}",
                                browser.cli_name,
                                browser.channel.canonical_name(),
                                path
                            );
                        }
                    }
                }
                eprintln!("System default: {}", inventory.system_default.display_name);
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
                            eprintln!(
                                "Browser '{}' ({}) is available at {} [{}]",
                                info.cli_name,
                                info.channel.canonical_name(),
                                path,
                                bundle_id
                            );
                        } else {
                            eprintln!(
                                "Browser '{}' ({}) is available at {}",
                                info.cli_name,
                                info.channel.canonical_name(),
                                path
                            );
                        }
                    } else {
                        eprintln!(
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

/// Handle the "profile" subcommand: list or show info about browser profiles.
///
/// If `browser` is None, the function attempts to resolve a browser named `"chrome"`.
/// Resolves the requested browser (honoring an optional `channel`) and then:
/// - ProfileAction::List: discovers profiles for that browser (optionally within `user_dir`) and
///   prints a human-readable listing or emits a JSON `ListProfilesResponse`.
/// - ProfileAction::Info { name }: finds a specific profile by name and prints detailed info or
///   emits a JSON `ProfileInfoResponse`.
///
/// Output format is chosen by `format`: `OutputFormat::Human` prints to stdout/stderr; the JSON
/// branch prints pretty-serialized responses to stdout. On resolution failures (browser not found,
/// profile discovery/find errors) the function logs an error in human mode and terminates the
/// process with exit code 1.
///
/// Side effects:
/// - Writes to stdout/stderr.
/// - May call `process::exit(1)` on errors.
///
/// Examples
///
/// ```rust,no_run
/// // Resolve inventory earlier (not shown) and call:
/// handle_profile_command(&inventory, Some("chrome".to_string()), None, None, ProfileAction::List, OutputFormat::Human);
/// ```
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
            } else {
                print_profile_error_json("profile-error", browser_name, error_msg);
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
                        eprintln!("{} profiles:", browser.display_name);
                        if profiles.is_empty() {
                            eprintln!("  (none)");
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

                                eprintln!(
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
                    } else {
                        print_profile_error_json(
                            "list-profiles",
                            browser.display_name.as_str(),
                            error_msg,
                        );
                    }
                    process::exit(1);
                }
            }
        }
        ProfileAction::Info { name } => {
            match ProfileManager::find_profile_in_directory(browser, &name, custom_dir) {
                Ok(profile) => {
                    if format == OutputFormat::Human {
                        eprintln!("Profile: {}", profile.display_name);
                        eprintln!("  Name: {}", profile.name);
                        eprintln!("  Path: {}", profile.path.display());
                        eprintln!(
                            "  Default: {}",
                            if profile.is_default { "Yes" } else { "No" }
                        );
                        if let Some(last_used) = &profile.last_used {
                            eprintln!("  Last used: {}", last_used);
                        }
                        eprintln!("  Browser: {}", browser.display_name);
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
                    } else {
                        print_profile_error_json(
                            "profile-info",
                            browser.display_name.as_str(),
                            error_msg,
                        );
                    }
                    process::exit(1);
                }
            }
        }
    }
}

/// Convert CLI profile arguments into a runtime ProfileOptions.
///
/// Chooses a ProfileType based on ProfileArgs:
/// - If `temp_profile` is set, attempts to create a temporary profile directory; on failure falls back to `Default` and appends a warning.
/// - If `user_dir` is provided, attempts to prepare that custom directory; on failure falls back to `Default` and appends a warning.
/// - If `guest` is set, returns `Guest`.
/// - If a named `profile` is provided, returns `Named(name)`.
/// - Otherwise returns `Default`.
///
/// The function may have side effects: creating a temporary profile directory via `ProfileManager::create_temp_profile`
/// or preparing a custom directory via `ProfileManager::prepare_custom_directory`. Any user-visible issues encountered
/// while performing those operations are appended to the provided `warnings` vector.
///
/// Returns a `ProfileOptions` with the selected `ProfileType` and an empty `custom_args` list.
///
/// # Examples
///
/// ```
/// let mut warnings = Vec::new();
/// let args = ProfileArgs {
///     temp_profile: false,
///     user_dir: None,
///     guest: false,
///     profile: None,
/// };
/// let opts = convert_profile_args(&args, &mut warnings);
/// assert!(matches!(opts.profile_type, ProfileType::Default));
/// assert!(warnings.is_empty());
/// ```
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

/// Convert CLI window argument flags into a WindowOptions value used for launches.
///
/// The returned `WindowOptions` mirrors the `new_window`, `incognito`, and `kiosk` flags
/// from the provided `WindowArgs`.
///
/// # Examples
///
/// ```
/// let args = WindowArgs { new_window: true, incognito: false, kiosk: false };
/// let opts = convert_window_args(&args);
/// assert!(opts.new_window && !opts.incognito && !opts.kiosk);
/// ```
fn convert_window_args(window_args: &WindowArgs) -> WindowOptions {
    WindowOptions {
        new_window: window_args.new_window,
        incognito: window_args.incognito,
        kiosk: window_args.kiosk,
    }
}

// Helper implementations
impl BrowserJson {
    /// Create a BrowserJson representation from a detected BrowserInfo.
    ///
    /// The returned JSON object copies the CLI-visible name, canonical channel name,
    /// a filesystem path (prefers bundle_path, falls back to executable), bundle ID,
    /// and whether this browser is the system default.
    ///
    /// # Parameters
    ///
    /// - `info`: browser discovery result; `channel` is used via its `canonical_name()`
    ///   and either `bundle_path` or `executable` is selected for `path`.
    /// - `is_default`: marks the resulting JSON as the system default browser when true.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Given a `BrowserInfo` named `info` and a boolean `is_default`:
    /// let json = BrowserJson::from_browser(&info, is_default);
    /// println!("{}", json.name);
    /// ```
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

    /// Create a BrowserJson representing the system default browser.
    ///
    /// Converts a SystemDefaultBrowser into the JSON-friendly BrowserJson:
    /// - uses the system display name as `name`
    /// - maps an optional `channel` to its canonical name string when present
    /// - maps an optional `path` to a display string when present
    /// - leaves `bundle_id` as `None` and sets `is_default` to `true`.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given a resolved `sys_default` value:
    /// let json = BrowserJson::from_system_default(&sys_default);
    /// assert!(json.is_default);
    /// ```
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
    /// Build a JSON-serializable representation of the given ProfileOptions.
    ///
    /// The returned `ProfileJson` contains:
    /// - `profile_type`: a string label ("default", "named", "custom", "temporary", or "guest"),
    /// - `name`: present only for `ProfileType::Named`,
    /// - `path`: present for `ProfileType::CustomDirectory` and `ProfileType::Temporary` (stringified via `Display`).
    ///
    /// # Examples
    ///
    /// ```
    /// use pathway::ProfileOptions;
    /// use pathway::ProfileType;
    ///
    /// // Named profile
    /// let opts = ProfileOptions { profile_type: ProfileType::Named("work".into()), custom_args: vec![] };
    /// let json = crate::ProfileJson::from_profile_options(&opts);
    /// assert_eq!(json.profile_type, "named");
    /// assert_eq!(json.name.as_deref(), Some("work"));
    ///
    /// // Default profile
    /// let opts = ProfileOptions { profile_type: ProfileType::Default, custom_args: vec![] };
    /// let json = crate::ProfileJson::from_profile_options(&opts);
    /// assert_eq!(json.profile_type, "default");
    /// assert!(json.name.is_none() && json.path.is_none());
    /// ```
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
    /// Create a JSON-serializable representation of window options.
    ///
    /// Converts a Pathway `WindowOptions` into the module's `WindowOptionsJson` shape
    /// by copying the `new_window`, `incognito`, and `kiosk` flags.
    ///
    /// # Examples
    ///
    /// ```
    /// let opts = WindowOptions { new_window: true, incognito: false, kiosk: false };
    /// let json = WindowOptionsJson::from_window_options(&opts);
    /// assert_eq!(json.new_window, true);
    /// assert_eq!(json.incognito, false);
    /// assert_eq!(json.kiosk, false);
    /// ```
    fn from_window_options(window_opts: &WindowOptions) -> Self {
        WindowOptionsJson {
            new_window: window_opts.new_window,
            incognito: window_opts.incognito,
            kiosk: window_opts.kiosk,
        }
    }
}

fn print_profile_error_json(action: &'static str, browser: &str, message: String) {
    let resp = ProfileErrorResponse {
        action,
        browser: browser.to_string(),
        message,
    };
    println!("{}", serde_json::to_string_pretty(&resp).unwrap());
}

fn print_launch_error_json(normalized_urls: &[String], results: &[ValidatedUrl], message: &str) {
    let response = LaunchJsonResponse {
        action: "launch",
        status: "error",
        urls: normalized_urls.to_vec(),
        url: normalized_urls.first().cloned(),
        validated: results.to_vec(),
        warnings: None,
        browser: None,
        profile: None,
        window_options: None,
        command: None,
        message: Some(message.to_string()),
    };
    println!("{}", serde_json::to_string_pretty(&response).unwrap());
}

#[allow(clippy::too_many_arguments)]
fn build_launch_json_response(
    status: &'static str,
    urls: &[String],
    results: &[ValidatedUrl],
    warnings: &[String],
    browser_json: Option<BrowserJson>,
    selected_browser: Option<&BrowserInfo>,
    profile_options: &ProfileOptions,
    window_options: &WindowOptions,
    command: Option<LaunchCommand>,
    message: Option<String>,
) -> LaunchJsonResponse {
    let include_opts = selected_browser.is_some();
    LaunchJsonResponse {
        action: "launch",
        status,
        urls: urls.to_vec(),
        url: urls.first().cloned(),
        validated: results.to_vec(),
        warnings: if warnings.is_empty() {
            None
        } else {
            Some(warnings.to_vec())
        },
        browser: browser_json,
        profile: if include_opts {
            Some(ProfileJson::from_profile_options(profile_options))
        } else {
            None
        },
        window_options: if include_opts {
            Some(WindowOptionsJson::from_window_options(window_options))
        } else {
            None
        },
        command,
        message,
    }
}

/// Returns a short human-readable description of the selected profile option suitable for appending
/// to a launch message (e.g., " with profile 'name'", " in guest mode").
///
/// Examples
///
/// ```no_run
/// use crate::{get_profile_description, ProfileOptions, ProfileType};
///
/// let def = ProfileOptions { profile_type: ProfileType::Default, ..Default::default() };
/// assert_eq!(get_profile_description(&def), "");
///
/// let named = ProfileOptions { profile_type: ProfileType::Named("work".into()), ..Default::default() };
/// assert_eq!(get_profile_description(&named), " with profile 'work'");
/// ```
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

/// Handle URL validation errors by generating appropriate error response
fn handle_url_validation_error(
    normalized_urls: &[String],
    results: &[ValidatedUrl],
    format: OutputFormat,
) {
    if format == OutputFormat::Json {
        let response = LaunchJsonResponse {
            action: "launch",
            status: "error",
            urls: normalized_urls.to_vec(),
            url: normalized_urls.first().cloned(),
            validated: results.to_vec(),
            warnings: None,
            browser: None,
            profile: None,
            window_options: None,
            command: None,
            message: Some("URL validation failed".to_string()),
        };
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    }
}

/// Generate browser resolution warnings
fn generate_browser_warnings(
    browser: &Option<String>,
    selected_browser: Option<&BrowserInfo>,
    requested_channel: Option<BrowserChannel>,
    inventory: &BrowserInventory,
    format: OutputFormat,
    is_fallback: bool,
) -> Vec<String> {
    let mut warnings = Vec::new();

    if is_fallback {
        debug_assert!(
            selected_browser.is_some(),
            "fallback should always resolve a browser"
        );
        let fallback_name = selected_browser
            .map(|b| b.display_name.as_str())
            .unwrap_or("<unreachable>");
        let warning = format!(
            "Using {} instead of system default (--no-system-default was specified)",
            fallback_name
        );
        if format == OutputFormat::Human {
            warn!("{}", warning);
        }
        warnings.push(warning);
    }

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
        warnings.push(warning);
    }

    warnings
}

/// Handle no-launch (dry-run) response generation
fn handle_no_launch_response(
    profile_options: &ProfileOptions,
    window_options: &WindowOptions,
    response_data: LaunchResponseData,
) {
    if response_data.format == OutputFormat::Human {
        if let Some(browser) = response_data.selected_browser {
            let profile_info = get_profile_description(profile_options);
            info!(
                "Launch skipped (--no-launch). Would launch in {}{}",
                browser.display_name.as_str(),
                profile_info
            );
        } else {
            info!(
                "Launch skipped (--no-launch). Would launch in {}",
                response_data.inventory.system_default.display_name.as_str()
            );
        }
    } else {
        let browser_json = response_data
            .selected_browser
            .map(|info| BrowserJson::from_browser(info, false))
            .unwrap_or_else(|| {
                BrowserJson::from_system_default(&response_data.inventory.system_default)
            });

        let response = build_launch_json_response(
            "skipped",
            response_data.normalized_urls,
            response_data.results,
            response_data.warnings,
            Some(browser_json),
            response_data.selected_browser,
            profile_options,
            window_options,
            None,
            Some("Launch skipped (--no-launch)".to_string()),
        );
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    }
}
