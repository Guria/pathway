use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use std::io;

pub fn setup_logging(verbose: bool, json_format: bool) {
    let env_filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    if json_format {
        tracing_subscriber::registry()
            .with(fmt::layer().json().with_writer(io::stderr))
            .with(env_filter)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(fmt::layer().with_writer(io::stderr))
            .with(env_filter)
            .init();
    }
}
