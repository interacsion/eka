use crate::cli::store::Detected;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, short, conflicts_with = "path")]
    bare: bool,

    #[command(subcommand)]
    init: InitStore,
}

#[derive(Subcommand, Debug, Clone)]
enum InitStore {
    Git,
}

mod error {
    use thiserror::Error as ThisError;
    #[derive(ThisError, Debug)]
    pub enum Error {}
}

pub(super) fn run(store: Option<Detected>, args: Args) -> Result<(), error::Error> {
    if let Some(store) = store {
        match (store, args.bare) {
            (Detected::Git(repo), true) => {
                let repo = repo.to_thread_local();
            }
            (Detected::Git(_repo), false) => todo!(),
        }
    } else {
        match args.init {
            InitStore::Git => {}
        }
    }
    Ok(())
}
