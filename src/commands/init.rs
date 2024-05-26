use dialoguer::{Input, Select};

use crate::{structs::{ModLoader, Modpack}, Result, MODRINTH};

pub async fn init() -> Result<()> {
    if Modpack::read().is_ok() {
        println!("This folder already has a modpack! \nrun `mcpack help` for help");
        return Ok(());
    }

    let modpack_name: String = Input::new()
        .with_prompt("Name your modpack")
        .interact_text()?;

    let modpack_author: String = Input::new()
        .with_prompt("Who is the modpack's author")
        .interact_text()?;
    
    let modpack_game_version = pick_game_version().await?;
    let modpack_loader = pick_loader().await?;

    Modpack::write(Modpack::new(modpack_name, modpack_author,  modpack_game_version, modpack_loader))?;

    Ok(())
}

async fn pick_game_version() -> Result<String> {
    let versions: Vec<String> = MODRINTH
        .list_game_versions()
        .await?
        .into_iter()
        .filter(|v| v.major) // no way to pick snapshots yet
        .map(|v| v.version)
        .collect();

    let game_version = Select::new()
        .with_prompt("Choose the game version")
        .items(&versions)
        .interact()?;

    Ok(versions[game_version].to_owned())
}

async fn pick_loader() -> Result<ModLoader> {
    // LexForge will likely be unavailable in 1.21 and generally isnt recommended
    // todo: check for game_version and disallow picking if game_version is above 1.21
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
