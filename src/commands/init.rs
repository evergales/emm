use console::style;
use dialoguer::{Input, Select};

use crate::{cli::InitArgs, error::Result, structs::pack::{ModLoader, Modpack, PackOptions, Versions}, util::versions::{get_latest_loader_version, minecraft::{get_latest_release, get_latest_snapshot, list_mc_versions, VersionType}}};

pub async fn init(args: InitArgs) -> Result<()> {
    if Modpack::path().is_file() {
        println!("{} \nrun `emm help` for help", style("This folder already has a modpack!").color256(166));
        return Ok(());
    }

    let name: String = match args.name {
        Some(name) => name,
        None => {
            Input::new()
            .with_prompt("Name your modpack")
            .interact_text()
            .unwrap()
        },
    };
   
    let game_version = if args.latest {
        get_latest_release().await?
    } else if args.latest_snapshot {
        get_latest_snapshot().await?
    } else {
        pick_game_version(args.show_snapshots).await?
    };

    let loader = match args.loader {
        Some(loader) => loader,
        None => pick_loader().await?,
    };

    let mut options = PackOptions::default();
    if loader == ModLoader::Quilt {
        // quilt is compatible with most fabric mods, so this should be default
        // putting this in acceptable_loaders instead of somewhere in compat checks makes it disableable
        options.acceptable_loaders = Some(vec![ModLoader::Fabric]);
    }

    Modpack::write(&Modpack {
        name,
        version: "0.1.0".into(),
        authors: args.authors.unwrap_or_default(),
        description: args.description,
        index_path: "./index".into(),
        options,
        versions: Versions {
            minecraft: game_version,
            loader,
            loader_version: "latest".into(),
        },
    })?;

    Ok(())
}

pub async fn pick_game_version(snapshots: bool) -> Result<String> {
    let filter = match snapshots {
        false => Some(VersionType::Release),
        true => None,
    };

    let versions: Vec<String> = list_mc_versions(filter).await?;

    let game_version = Select::new()
        .with_prompt("Choose the game version")
        .items(&versions)
        .interact()
        .unwrap();

    Ok(versions[game_version].to_owned())
}

async fn pick_loader() -> Result<ModLoader> {
    let loader_picker = Select::new()
        .with_prompt("Choose the modloader")
        .items(&["Fabric", "Quilt", "Forge", "NeoForge"])
        .interact()
        .unwrap();

    let loader = match loader_picker {
        0 => ModLoader::Fabric,
        1 => ModLoader::Quilt,
        2 => ModLoader::Forge,
        3 => ModLoader::NeoForge,
        _ => unreachable!()
        
    };

    Ok(loader)
}