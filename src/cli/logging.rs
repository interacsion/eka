use super::{Args, LogArgs};

use clap::Parser;
use std::str::FromStr;
use tracing_error::ErrorLayer;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::fmt;
use tracing_subscriber::Layer;
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

use tracing_appender::non_blocking::WorkerGuard;
pub fn init_global_subscriber(args: LogArgs) -> (WorkerGuard, bool) {
    let log_level = get_log_level(args);

    let env_filter = EnvFilter::from_default_env().add_directive(log_level.into());

    let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stderr());

    let mut ansi: bool = true;

    use std::io::IsTerminal;
    let fmt = if std::io::stderr().is_terminal() {
        fmt::layer()
            .without_time()
            .with_writer(non_blocking)
            .boxed()
    } else {
        ansi = false;
        fmt::layer()
            .with_ansi(ansi)
            .json()
            .with_writer(non_blocking)
            .boxed()
    };

    tracing_subscriber::registry()
        .with(fmt)
        .with(env_filter)
        .with(ErrorLayer::default())
        .init();

    if log_level == LevelFilter::TRACE {
        let _ = Args::parse();
    }

    (guard, ansi)
}
