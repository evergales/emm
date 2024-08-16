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
        ImportCommmands::Modrinth(args) => modrinth::import_modrinth(args).await,
        ImportCommmands::Curseforge(args) => todo!(),
        ImportCommmands::Packwiz(args) => packwiz::import_packwiz(args).await,
    }
}