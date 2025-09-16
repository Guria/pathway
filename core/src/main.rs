use clap::Parser;
use pathway::{logging, validate_url, ValidatedUrl, ValidationStatus};
use serde_json;
use std::process;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(author, version, about = "URL routing agent for Pathway", long_about = None)]
struct Args {
    /// URLs to validate
    #[arg(required = true)]
    urls: Vec<String>,

    /// Enable debug logging
    #[arg(short, long)]
    verbose: bool,

    /// Output format
    #[arg(long, value_enum, default_value = "human")]
    format: OutputFormat,

    /// Validate without opening
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

fn main() {
    let args = Args::parse();

    // Set up logging based on format
    // For JSON output, we don't want any logging to interfere
    if matches!(args.format, OutputFormat::Human) {
        logging::setup_logging(args.verbose, false);
    }

    let mut results = Vec::new();
    let mut has_error = false;

    for (index, url) in args.urls.iter().enumerate() {
        match validate_url(url) {
            Ok(validated) => {
                results.push(validated.clone());
                
                if matches!(args.format, OutputFormat::Human) {
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
                            validated.normalized,
                            validated.scheme
                        );
                    }
                }
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
                
                if matches!(args.format, OutputFormat::Human) {
                    error!("URL {}: {}", index + 1, e);
                }
            }
        }
    }

    // Output results based on format
    match args.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(&results).unwrap());
        }
        OutputFormat::Human => {
            // Already logged above
        }
    }

    // Exit with appropriate code
    if has_error {
        process::exit(1);
    }
}
