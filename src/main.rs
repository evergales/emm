mod structs;
mod commands;
mod util;

use std::env;
use ferinth::Ferinth;
use furse::Furse;
use lazy_static::lazy_static;
use clap::Parser;
use commands::{export, migrate, Commands};

lazy_static! {
    pub static ref MODRINTH: Ferinth = Ferinth::new("eg-mc", Some("0.1.0"), None, None).unwrap();
    pub static ref CURSEFORGE: Furse = {
        let key = env::var("CURSEFORGE_API_KEY").unwrap_or("$2a$10$Grlqtes/CrLoTgnvg174H.BKRX8caplGh0o1dOwxhhMWAgv.2J9cC".into());
        Furse::new(&key)
    };
}

#[derive(thiserror::Error, Debug)]
#[error("{}", .0)]
pub enum Error {
    #[error("Error while parsing, {0}")]
    Parse(String),

    #[error("The folder you're in doesnt have a modpack, create one with emm init")]
    Uninitialized,

    #[error("{0}")]
    Other(String),

    Modrinth(#[from] ferinth::Error),
    Curseforge(#[from] furse::Error),
    Dialoguer(#[from] dialoguer::Error),
    Io(#[from] std::io::Error),
    JoinError(#[from] tokio::task::JoinError),
    Reqwest(#[from] reqwest::Error)
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Err(err) = match args.command {
        Commands::Init => commands::init::init().await,
        Commands::Add { mods, ignore_version, ignore_loader } => commands::add::add_mod(mods, ignore_version, ignore_loader).await,
        Commands::Remove { mods } => commands::remove::remove_mod(mods).await,
        Commands::Pin { m } => commands::pin::pin(m).await,
        Commands::Unpin { m } => commands::unpin::unpin(m).await,
        Commands::Update => commands::update::update().await,
        Commands::Migrate { subcommand } => match subcommand {
            migrate::Commands::Loader => migrate::loader::migrate_loader().await,
            migrate::Commands::Minecraft => migrate::minecraft::migrate_minecraft().await,
        },
        Commands::Export { subcommand } => match subcommand {
            commands::export::Commands::Modrinth { overrides_path } => export::modrinth::export_modrinth(overrides_path).await,
        }
    } {
        eprintln!("{}", err)
    }

}
