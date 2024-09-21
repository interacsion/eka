use super::{Args, LogArgs};

use clap::Parser;
use serde::Serialize;
use std::error;
use std::fmt::Display;
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

    if log_level == LevelFilter::TRACE {
        let _ = Args::parse();
    }
}

pub(super) trait LogValue {
    fn as_json(&self) -> String
    where
        Self: Serialize;
    fn trim_whitespace(&self) -> String
    where
        Self: Display;
    fn log_err<T, E>(self) -> Result<T, E>
    where
        Self: Sized + Into<Result<T, E>>,
        E: Display + error::Error + LogValue;
}

impl<T> LogValue for T {
    fn as_json(&self) -> String
    where
        Self: Serialize,
    {
        serde_json::to_string(self).unwrap_or_else(|_| "null".to_string())
    }
    fn trim_whitespace(&self) -> String
    where
        Self: Display,
    {
        self.to_string().trim().to_owned()
    }
    fn log_err<P, E>(self) -> Result<P, E>
    where
        Self: Sized + Into<Result<P, E>>,
        E: Display + error::Error + LogValue,
    {
        self.into().map_err(log_error)
    }
}

pub(super) fn log_error<E>(e: E) -> E
where
    E: Display + error::Error + LogValue,
{
    tracing::error!(message = %e.trim_whitespace());
    e
}
