use super::args::LogArgs;
use serde::Serialize;
use std::str::FromStr;
use tracing_error::ErrorLayer;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::fmt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn get_log_level(args: LogArgs) -> LevelFilter {
    if args.quiet {
        return LevelFilter::ERROR;
    }

    if let Ok(rust_log) = std::env::var(EnvFilter::DEFAULT_ENV) {
        if let Ok(level) = LevelFilter::from_str(&rust_log) {
            return level;
        }
    }

    match args.verbosity {
        0 => LevelFilter::WARN,
        1 => LevelFilter::INFO,
        2 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    }
}

pub fn init_logger(args: LogArgs) {
    let log_level = get_log_level(args);

    let env_filter = EnvFilter::from_default_env().add_directive(log_level.into());

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(env_filter)
        .with(ErrorLayer::default())
        .init();
}

pub trait LogValue {
    fn as_json(&self) -> String;
}

impl<T: Serialize> LogValue for T {
    fn as_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "null".to_string())
    }
}
