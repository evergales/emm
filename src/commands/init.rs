use console::style;
use dialoguer::{Input, Select};

use crate::{structs::{ModLoader, Modpack}, util::versions::{get_latest_loader_version, minecraft::{get_latest_release, get_latest_snapshot, list_mc_versions, VersionType}}, Result};

pub async fn init(name: Option<String>, description: Option<String>, authors: Option<Vec<String>>, latest: bool, loader: Option<ModLoader>, latest_snapshot: bool, show_snapshots: bool) -> Result<()> {
    if Modpack::read().is_ok() {
        println!("{} \nrun `emm help` for help", style("This folder already has a modpack!").color256(166));
        return Ok(());
    }

    let modpack_name: String = match name {
        Some(name) => name,
        None => {
            Input::new()
            .with_prompt("Name your modpack")
            .interact_text()?
        },
    };
   
    let modpack_game_version = if latest {
        get_latest_release().await?
    } else if latest_snapshot {
        get_latest_snapshot().await?
    } else {
        pick_game_version(show_snapshots).await?
    };

    let modpack_loader = match loader {
        Some(loader) => loader,
        None => pick_loader().await?,
    };

    let loader_version = get_latest_loader_version(&modpack_loader, &modpack_game_version).await?;

    Modpack::write(&Modpack::new(
        modpack_name.clone(),
        if authors.is_some() { authors.unwrap() } else {vec!["you!".to_owned()]},
        Some(description.unwrap_or(format!("the {} pack!", modpack_name))),
        "0.1.0".into(),
        modpack_game_version,
        modpack_loader,
        loader_version
    ))?;

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
        .interact()?;

    Ok(versions[game_version].to_owned())
}

async fn pick_loader() -> Result<ModLoader> {
    let loader_picker = Select::new()
        .with_prompt("Choose the modloader")
        .items(&["Fabric", "Quilt", "Forge", "NeoForge"])
        .interact()?;

    let loader = match loader_picker {
        0 => ModLoader::Fabric,
        1 => ModLoader::Quilt,
        2 => ModLoader::Forge,
        3 => ModLoader::NeoForge,
        _ => unreachable!()
        
    };

    Ok(loader)
}
