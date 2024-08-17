use crate::{cli::{MigrateArgs, MigrateCommands}, error::Result};

pub mod minecraft;

pub async fn run(args: MigrateArgs) -> Result<()> {
    match args.subcommand {
        MigrateCommands::Minecraft(args) => minecraft::migrate_minecraft(args).await,
        MigrateCommands::Loader => todo!(),
    }
}