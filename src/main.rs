use clap::Parser;
use config::ConfigLoadSaveError;
use std::{path::PathBuf, process};
use thiserror::Error;

use crate::config::Config;

mod archived_message;
mod archiver;
mod config;
mod mong;
mod util;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{err}");
        process::exit(1);
    }
}

#[derive(Debug, Error)]
pub enum MainError {
    #[error(transparent)]
    Serenity(#[from] serenity::Error),

    #[error(transparent)]
    Mongodb(#[from] mongodb::error::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("Failed to load the config: {0}")]
    Config(#[from] ConfigLoadSaveError),
}

#[derive(Debug, clap::Parser)]
struct Args {
    #[arg(value_enum)]
    pub mode: Mode,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
enum Mode {
    ArchiveNewMessages,
}

async fn run() -> Result<(), MainError> {
    let args = Args::parse();

    let config = Config::load(&PathBuf::from("./config.toml")).await?;

    match args.mode {
        Mode::ArchiveNewMessages => archiver::run(config).await,
    }
}
