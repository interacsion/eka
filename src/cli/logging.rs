use std::str::FromStr;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::fmt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn get_log_level(verbosity: u8) -> LevelFilter {
    if let Ok(rust_log) = std::env::var("RUST_LOG") {
        if let Ok(level) = LevelFilter::from_str(&rust_log) {
            return level;
        }
    }

    match verbosity {
        0 => LevelFilter::WARN,
        1 => LevelFilter::INFO,
        2 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    }
}

pub fn init_logger(verbosity: u8) {
    let log_level = get_log_level(verbosity);

    let env_filter = EnvFilter::from_default_env().add_directive(log_level.into());

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(env_filter)
        .init();
}
