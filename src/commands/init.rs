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
    
    let modpack_game_version = pick_game_version().await?;
    let modpack_loader = pick_loader().await?;

    /*
    todo: properly get mod_loader versions!!
    get from maven xml files!
    https://maven.fabricmc.net/net/fabricmc/fabric-loader/maven-metadata.xml
    https://maven.quiltmc.org/repository/release/org/quiltmc/quilt-loader/maven-metadata.xml

    neoforge & forge have specific loaders per mc version, needs to be parsed properly
    https://maven.neoforged.net/releases/net/neoforged/forge/maven-metadata.xml
    https://files.minecraftforge.net/maven/net/minecraftforge/forge/maven-metadata.xml
    */

    Modpack::write(Modpack::new(modpack_name, vec!["you!".to_string()],  Some("my awesome modpack!".to_string()), modpack_game_version, modpack_loader, String::new()))?;

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
