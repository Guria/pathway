use clap::{Parser, ValueEnum};
use pathway::{
    available_tokens, detect_inventory, find_browser, launch, logging, validate_url,
    BrowserChannel, BrowserInfo, BrowserInventory, LaunchCommand, LaunchTarget,
    SystemDefaultBrowser, ValidatedUrl, ValidationStatus,
};
use serde::Serialize;
use std::process;
use tracing::{error, info, warn};

#[derive(Parser, Debug)]
#[command(author, version, about = "URL routing agent for Pathway", long_about = None)]
struct Args {
    /// URLs to open
    #[arg(required_unless_present_any = ["list_browsers", "check_browser"], num_args = 0..)]
    urls: Vec<String>,

    /// Enable debug logging
    #[arg(short, long)]
    verbose: bool,

    /// Output format
    #[arg(long, value_enum, default_value = "human")]
    format: OutputFormat,

    /// Specify browser (chrome, firefox, safari, etc.)
    #[arg(long)]
    browser: Option<String>,

    /// Browser channel (stable, beta, dev, canary, nightly)
    #[arg(long, value_enum)]
    channel: Option<BrowserChannelArg>,

    /// List all detected browsers and exit
    #[arg(long)]
    list_browsers: bool,

    /// Check if a specific browser is available
    #[arg(long)]
    check_browser: Option<String>,

    /// Force use of system default browser
    #[arg(long)]
    system_default: bool,

    /// Validate URL but don't launch
    #[arg(long, alias = "dry-run")]
    no_launch: bool,
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
            is_default: true,
        }
    }
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

fn main() {
    let args = Args::parse();

    if args.format == OutputFormat::Human {
        logging::setup_logging(args.verbose, false);
    }

    let inventory = detect_inventory();

    if args.list_browsers {
        handle_list_browsers(&inventory, args.format);
        return;
    }

    if let Some(target) = args.check_browser.as_deref() {
        handle_check_browser(
            &inventory,
            target,
            args.channel.map(Into::into),
            args.format,
        );
        return;
    }

    if args.urls.is_empty() {
        error!("No URLs provided");
        process::exit(1);
    }

    let mut results = Vec::new();
    let mut has_error = false;

    for (index, url) in args.urls.iter().enumerate() {
        match validate_url(url) {
            Ok(validated) => {
                if args.format == OutputFormat::Human {
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

                if args.format == OutputFormat::Human {
                    error!("URL {}: {}", index + 1, e);
                }
            }
        }
    }

    let normalized_urls: Vec<String> = results.iter().map(|url| url.normalized.clone()).collect();

    if has_error {
        if args.format == OutputFormat::Json {
            let response = LaunchJsonResponse {
                action: "launch",
                status: "error",
                urls: normalized_urls.clone(),
                url: normalized_urls.first().cloned(),
                validated: results.clone(),
                warnings: None,
                browser: None,
                command: None,
                message: Some("URL validation failed".to_string()),
            };
            println!("{}", serde_json::to_string_pretty(&response).unwrap());
        }
        process::exit(1);
    }

    let requested_channel = args.channel.map(Into::into);
    let mut warnings = Vec::new();
    let selected_browser = if args.system_default {
        None
    } else if let Some(name) = args.browser.as_deref() {
        match find_browser(&inventory.browsers, name, requested_channel) {
            Some(info) => Some(info),
            None => {
                let mut warning = format!("Browser '{}' not found", name);
                if let Some(channel) = requested_channel {
                    warning.push_str(&format!(" (channel: {})", channel.canonical_name()));
                }
                warning.push_str(&format!(
                    ". Available browsers: {}",
                    available_tokens(&inventory.browsers).join(", ")
                ));

                if args.format == OutputFormat::Human {
                    warn!("{}", warning);
                }
                warnings.push(warning);
                None
            }
        }
    } else {
        None
    };

    let launch_target = selected_browser
        .map(LaunchTarget::Browser)
        .unwrap_or(LaunchTarget::SystemDefault);

    if args.no_launch {
        if args.format == OutputFormat::Human {
            if let Some(browser) = selected_browser {
                info!(
                    "Launch skipped (--no-launch). Would launch in {}",
                    browser.display_name.as_str()
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
                command: None,
                message: Some("Launch skipped (--no-launch)".to_string()),
            };
            println!("{}", serde_json::to_string_pretty(&response).unwrap());
        }
        return;
    }

    match launch(launch_target, &normalized_urls) {
        Ok(outcome) => {
            if args.format == OutputFormat::Human {
                if let Some(browser) = selected_browser {
                    info!(
                        "Launching in {}: {}",
                        browser.display_name,
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
                    command: Some(outcome.command.clone()),
                    message: None,
                };
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            }
        }
        Err(err) => {
            let message = format!("Failed to launch browser: {}", err);
            if args.format == OutputFormat::Human {
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
                    command: None,
                    message: Some(message.clone()),
                };
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            }
            process::exit(1);
        }
    }
}

fn handle_list_browsers(inventory: &BrowserInventory, format: OutputFormat) {
    match format {
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
                    println!(
                        "  {} ({}) - {}",
                        browser.cli_name,
                        browser.channel.canonical_name(),
                        path
                    );
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
    }
}

fn handle_check_browser(
    inventory: &BrowserInventory,
    name: &str,
    channel: Option<BrowserChannel>,
    format: OutputFormat,
) {
    let result = find_browser(&inventory.browsers, name, channel);

    match format {
        OutputFormat::Human => {
            if let Some(info) = result {
                println!(
                    "Browser '{}' ({}) is available at {}",
                    info.cli_name,
                    info.channel.canonical_name(),
                    info.bundle_path
                        .as_ref()
                        .or(info.executable.as_ref())
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "(unknown path)".to_string())
                );
            } else {
                println!(
                    "Browser '{}' not found. Available browsers: {}",
                    name,
                    available_tokens(&inventory.browsers).join(", ")
                );
                process::exit(1);
            }
        }
        OutputFormat::Json => {
            let response = CheckJsonResponse {
                action: "check-browser",
                browser: name.to_string(),
                channel,
                available: result.is_some(),
                resolved: result.cloned(),
                message: if result.is_none() {
                    Some(format!(
                        "Browser '{}' not found. Available browsers: {}",
                        name,
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
