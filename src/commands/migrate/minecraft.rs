use std::sync::{Arc, Mutex};

use dialoguer::Confirm;
use tokio::task::JoinSet;

use crate::{
    commands::init::pick_game_version, structs::{Index, Modpack}, util::{get_latest_loader_version, join_all, seperate_mods_by_platform}, Result, CURSEFORGE, MODRINTH
};

pub async fn migrate_minecraft() -> Result<()> {
    let mut modpack = Modpack::read()?;
    let mc_version = pick_game_version().await?;
    let loader_version = get_latest_loader_version(&modpack.versions.mod_loader, &modpack.versions.minecraft).await?;

    let index = Index::read()?;
    let (mr_mods, cf_mods) = seperate_mods_by_platform(index.mods.clone()).await?;
    let incompatible_mods = Arc::new(Mutex::new(Vec::new()));

    let mut mr_tasks: JoinSet<crate::Result<()>> = JoinSet::new();
    for m in mr_mods {
        let mc_version = mc_version.clone();
        let modpack = modpack.clone();
        let incompatible_mods = Arc::clone(&incompatible_mods);

        let task = async move {
            let versions = MODRINTH.list_versions(&m.id).await?;
            if !versions.into_iter().any(|v| {
                v.game_versions.contains(&mc_version)
                && v.loaders.contains(&modpack.versions.mod_loader.to_string().to_lowercase())
            }) {
                incompatible_mods.lock().unwrap().push(m.name)
            };

            Ok(())
        };
        
        mr_tasks.spawn(task);
    }

    join_all(mr_tasks).await?;

    let mut cf_tasks: JoinSet<crate::Result<()>> = JoinSet::new();
    for m in cf_mods {
        let mc_version = mc_version.clone();
        let modpack = modpack.clone();
        let incompatible_mods = Arc::clone(&incompatible_mods);

        let task = async move {
            let files = CURSEFORGE.get_mod_files(m.id).await?;
            if !files.into_iter().any(|f| 
                f.is_available
                && f.game_versions.contains(&modpack.versions.mod_loader.to_string())
                && f.game_versions.contains(&mc_version)
            ) {
                incompatible_mods.lock().unwrap().push(m.name)
            };

            Ok(())
        };

        cf_tasks.spawn(task);
    }

    join_all(cf_tasks).await?;

    let mut incompatibles = incompatible_mods.lock().unwrap().to_owned();
    drop(incompatible_mods);

    println!("{}/{} mods can be migrated to {}", index.mods.len() - incompatibles.len(), index.mods.len(), mc_version);
    if !incompatibles.is_empty() {
        println!("compatible versions not found for:", );
        incompatibles.sort();
        incompatibles.into_iter().for_each(|m| println!("{m}"));
    }

    println!();
    if !Confirm::new()
        .with_prompt("migrate to new version?")
        .interact()? 
    {
        return Ok(());
    }

    modpack.versions.minecraft = mc_version;
    modpack.versions.loader_version = loader_version;
    Modpack::write(&modpack)?;

    println!("run 'emm update' to update your mods");
    Ok(())
}
