use std::{sync::{Arc, Mutex}, time::Duration};
use ferinth::structures::version::LatestVersionBody;
use furse::structures::file_structs::File;
use indicatif::ProgressBar;
use tokio::task::JoinSet;

use crate::{
    structs::{Index, Mod, ModPlatform, Modpack}, util::{join_all, primary_file, seperate_mods_by_platform}, Result, CURSEFORGE, MODRINTH
};

pub async fn update() -> Result<()> {
    let modpack = Modpack::read()?;
    let mut index = Index::read()?;
    // filter out pinned mods so they dont get updated
    index.mods.retain(|m| !m.pinned); 

    let progress = ProgressBar::new_spinner().with_message("Updating mods");
    progress.enable_steady_tick(Duration::from_millis(100));

    let (index_mr_mods, index_cf_mods) = seperate_mods_by_platform(index.mods.clone()).await?;

    if !index_mr_mods.is_empty() {
        progress.set_message("Finding modrinth updates");
    }

    let mr_verions = MODRINTH
        .get_multiple_versions(
            index_mr_mods
                .iter()
                .map(|m| m.version.as_str())
                .collect::<Vec<&str>>()
                .as_slice(),
        )
        .await?;
    let mr_version_hashes: Vec<String> =  mr_verions.into_iter().map(|v| primary_file(v.files).hashes.sha1).collect();

    // include these for: shader, datapack, resourcepack support when getting latest versions
    let other_supported_loaders: Vec<String> = vec!["iris", "canvas", "optifine", "datapack", "minecraft"].into_iter().map(|s| s.into()).collect();

    // returns a HashMap<inputed-hash, latest-version>
    // I didnt notice that for a while and did stupid stuff :D
    let latest_mr_versions = MODRINTH.latest_versions_from_hashes(
            mr_version_hashes,
            LatestVersionBody {
                loaders: vec![modpack.versions.mod_loader.to_string().to_lowercase()].into_iter().chain(other_supported_loaders).collect(),
                game_versions: vec![modpack.versions.minecraft.clone()],
            },
        )
        .await?;

    let collected_cf_versions = Arc::new(Mutex::new(Vec::new()));
    let mut tasks: JoinSet<crate::Result<()>> = JoinSet::new();

    if !index_cf_mods.is_empty() {
        progress.set_message("Finding curseforge updates");
    }
    for cf_mod in index_cf_mods {
        let collected_cf_versions = collected_cf_versions.clone();
        let modpack = modpack.clone();
        
        let task = async move {
            let files = CURSEFORGE.get_mod_files(cf_mod.id).await?;

            let compatibles = files.into_iter().filter(|f| 
                    f.is_available
                    && f.game_versions.contains(&modpack.versions.mod_loader.to_string())
                    && f.game_versions.contains(&modpack.versions.minecraft)
                ).collect::<Vec<File>>();
    
            let latest = compatibles.into_iter().max_by_key(|f| f.file_date);
            if let Some(latest) = latest {
                collected_cf_versions.lock().unwrap().push(latest);
            }
            
            Ok(())
        };

        tasks.spawn(task);
    };

    join_all(tasks).await?;

    // get out of the Arc<Mutex<>>
    let latest_cf_versions = Arc::try_unwrap(collected_cf_versions).unwrap().into_inner().unwrap();

    progress.finish_and_clear();

    // pair of Mod and version id/hash
    let to_update: Vec<(Mod, String)> = index.mods.into_iter().filter_map(|i| {
        match i.platform {
            ModPlatform::Modrinth => {
                let latest_version = latest_mr_versions.values().find(|v| v.project_id == i.id).unwrap();
                if latest_version.id != i.version {
                    return Some((i, latest_version.id.to_owned()));
                }
            },
            ModPlatform::CurseForge => {
                let id = i.id.parse::<i32>().unwrap();
                let latest_version = latest_cf_versions.iter().find(|v| v.mod_id == id).unwrap();
                if latest_version.id != i.version.parse::<i32>().unwrap() {
                    return Some((i, latest_version.id.to_string()));
                }
            },

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