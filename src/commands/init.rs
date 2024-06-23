use dialoguer::{Input, Select};

use crate::{structs::{ModLoader, Modpack}, util::get_latest_loader_version, Result, MODRINTH};

pub async fn init() -> Result<()> {
    if Modpack::read().is_ok() {
        println!("This folder already has a modpack! \nrun `emm help` for help");
        return Ok(());
    }

    let modpack_name: String = Input::new()
        .with_prompt("Name your modpack")
        .interact_text()?;
    
    let modpack_game_version = pick_game_version().await?;
    let modpack_loader = pick_loader().await?;

    let loader_version = get_latest_loader_version(&modpack_loader, &modpack_game_version).await?;

    Modpack::write(&Modpack::new(modpack_name, vec!["you!".to_string()], Some("my awesome modpack!".to_string()), "0.1.0".into(), modpack_game_version, modpack_loader, loader_version))?;

    Ok(())
}

pub async fn pick_game_version() -> Result<String> {
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
