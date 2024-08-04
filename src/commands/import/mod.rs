use crate::{cli::{ImportArgs, ImportCommmands}, error::Result};

pub mod modrinth;

pub async fn run(args: ImportArgs) -> Result<()> {
    match args.subcommand {
        ImportCommmands::Modrinth { path } => modrinth::import_modrinth(path).await,
        ImportCommmands::Curseforge { path } => todo!(),
    }
}