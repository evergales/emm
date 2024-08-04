use std::env;
use api::{curseforge::CurseAPI, github::GithubApi, modrinth::ModrinthAPI};
use clap::Parser;
use cli::{Args, Commands};
use lazy_static::lazy_static;

mod api;
mod cli;
mod commands;
mod error;
mod structs;
mod util;

lazy_static! {
    pub static ref GITHUB: GithubApi = GithubApi::default();
    pub static ref MODRINTH: ModrinthAPI = ModrinthAPI::new(&format!("evergales/emm/{} (discord: evergales)", env!("CARGO_PKG_VERSION")));
    pub static ref CURSEFORGE: CurseAPI = {
        let key = env::var("CURSEFORGE_API_KEY").unwrap_or("$2a$10$Grlqtes/CrLoTgnvg174H.BKRX8caplGh0o1dOwxhhMWAgv.2J9cC".into());
        CurseAPI::new(&key)
    };
}

#[tokio::main]
async fn main() {
    // hi there!
    if let Err(err) = match Args::parse().subcommand {
        Commands::Init(args) => commands::init::init(args).await,
        Commands::Add(args) => commands::add::add(args).await,
        Commands::Remove(args) => commands::remove::remove(args).await,
        Commands::Update => commands::update::update().await,
        Commands::Import(args) => commands::import::run(args).await,
        Commands::Export(args) => commands::export::run(args).await,
        /*
        Commands::Pin { m, version } => todo!(),
        Commands::Unpin { m } => todo!(),
        Commands::List { verbose } => todo!(),
        Commands::Import { subcommand } => todo!(),
        Commands::Export { subcommand } => todo!(),
        Commands::Migrate { subcommand } => todo!(),
        Commands::Completion { shell } => todo!(),
        */
        _ => unimplemented!()
    } {
        eprintln!("{err:#?}")
    }
}
