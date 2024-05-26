use std::sync::{Arc, Mutex};
use ferinth::structures::version::LatestVersionBody;
use furse::structures::file_structs::File;
use tokio::task::JoinSet;

use crate::{
    modpack::seperate_mods_by_platform, structs::{Index, Mod, Modpack}, Result, CURSEFORGE, MODRINTH
};

pub async fn update() -> Result<()> {
    let modpack = Modpack::read()?;
    let mut index = Index::read()?;
    // filter out pinned mods so they dont get updated
    index.mods.retain(|m| m.pinned.is_none() || m.pinned == Some(false)); 

    let (index_mr_mods, index_cf_mods) = seperate_mods_by_platform(index.mods.clone()).await?;

    println!("finding updated versions..");

    // returns a HashMap<inputed-hash, latest-version>
    // I didnt notice that for a while and did stupid stuff :D
    let latest_mr_versions = MODRINTH.latest_versions_from_hashes(
            index_mr_mods.into_iter().map(|m| m.version).collect(),
            LatestVersionBody {
                loaders: vec![modpack.mod_loader.to_string().to_lowercase()],
                game_versions: vec![modpack.game_version.clone()],
            },
        )
        .await?;

    let collected_cf_versions = Arc::new(Mutex::new(Vec::new()));
    let mut tasks: JoinSet<crate::Result<()>> = JoinSet::new();

    for cf_mod in index_cf_mods {
        let collected_cf_versions = Arc::clone(&collected_cf_versions);
        let modpack = modpack.clone();
        
        let task = async move {
            let files = CURSEFORGE.get_mod_files(cf_mod.id).await?;

            let compatibles = files.into_iter().filter(|f| 
                    f.is_available
                    && f.game_versions.contains(&modpack.mod_loader.to_string())
                    && f.game_versions.contains(&modpack.game_version)
                ).collect::<Vec<File>>();
    
            let latest = compatibles.into_iter().max_by_key(|f| f.file_date).unwrap();
            collected_cf_versions.lock().unwrap().push(latest);
            
            Ok(())
        };

        tasks.spawn(task);
    };

    while let Some(res) = tasks.join_next().await {
        let _ = res?;
    }

    // get out of the Arc<Mutex<>>
    let latest_cf_versions = collected_cf_versions.lock().unwrap().clone();
    drop(collected_cf_versions);

    // pair of Mod and version id/hash
    let to_update: Vec<(Mod, String)> = index.mods.into_iter().filter_map(|i| {
        if i.modrinth_id.is_some() {
            let latest_hash = latest_mr_versions.get(&i.version)?.files.iter().find(|f| f.primary).unwrap().hashes.sha1.to_owned();
            if latest_hash != i.version {
                return Some((i, latest_hash));
            }
        }
        if i.curseforge_id.is_some() && !latest_cf_versions.iter().any(|v| v.id == i.version.parse::<i32>().unwrap()) {
            let new_version = latest_cf_versions.iter().find(|v| v.mod_id == i.curseforge_id.unwrap()).unwrap().id;
            return Some((i, new_version.to_string()));
        }

        None
    }).collect();

    if to_update.is_empty() {
        println!("No new updates found!");
        return Ok(());
    }

    let mut new_index = Index::read()?;
    for version in to_update {
        println!("Updating {}", version.0.name);
        let idx = new_index.mods.iter().position(|m| version.0 == *m).unwrap();
        new_index.mods[idx].version = version.1
    }
    Index::write(&new_index)?;
    Ok(())
}