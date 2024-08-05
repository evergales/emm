use std::path::PathBuf;

use crate::{cli::{ExportArgs, ExportCommands}, error::Result, structs::{index::ProjectType, pack::{Modpack, PackOptions}}};

pub mod modrinth;
pub mod curseforge;
pub mod packwiz;

pub async fn run(args: ExportArgs) -> Result<()> {
    match args.subcommand {
        ExportCommands::Modrinth { overrides_path } => modrinth::export_modrinth(overrides_path).await,
        ExportCommands::Curseforge { overrides_path } => todo!(),
        ExportCommands::Packwiz { export_path } => packwiz::export_packwiz(export_path).await
    }
}

impl ProjectType {
    pub fn export_folder(&self, options: PackOptions) -> PathBuf {
        match self {
            Self::Mod => options.mods_output.unwrap_or("mods".into()),
            Self::Shader => options.shaders_output.unwrap_or("shaderpacks".into()),
            Self::Datapack => options.shaders_output.unwrap_or("datapacks".into()),
            Self::Resourcepack => options.resourcepacks_output.unwrap_or("resourcepacks".into()),
            _ => "unknown".into()
        }
    }
}