use dialoguer::Confirm;

use crate::{cli::{ImportArgs, ImportCommmands}, error::Result, structs::pack::Modpack};

pub mod modrinth;
pub mod curseforge;
pub mod packwiz;

pub async fn run(args: ImportArgs) -> Result<()> {
    if Modpack::read().is_ok() {
        let confirm = Confirm::new()
            .with_prompt("Importing will overwrite your current modpack, continue?")
            .interact()
            .unwrap();

        if !confirm {
            return Ok(());
        }
    }
    match args.subcommand {
        ImportCommmands::Modrinth { path } => modrinth::import_modrinth(path).await,
        ImportCommmands::Curseforge { path } => todo!(),
        ImportCommmands::Packwiz { source } => packwiz::import_packwiz(source).await,
    }
}