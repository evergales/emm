use std::{env, fs::{self, File}, io::Read, path::PathBuf, time::Duration};

use dialoguer::Confirm;
use indicatif::ProgressBar;
use serde_json::from_str;
use sha1::{Digest, Sha1};
use zip::ZipArchive;

use crate::{structs::{Index, Mod, ModLoader, ModPlatform, Modpack, ModpackAbout, ModpackVersions}, util::primary_file, Error, Result, CURSEFORGE, MODRINTH};

use super::CfManifest;

pub async fn import_curseforge(cfpack_path: PathBuf) -> Result<()> {
    if Modpack::read().is_ok() {
        let confirm = Confirm::new()
            .with_prompt("Importing will overwrite your current modpack, continue?")
            .interact()?;

        if !confirm {
            return Ok(());
        }
    }

    if !cfpack_path.is_file() || cfpack_path.extension().unwrap_or_default() != "zip" {
        return Err(Error::Other("The path you provided is not a curseforge pack file".to_string()));
    }

    let progress = ProgressBar::new_spinner().with_message("Reading mrpack file");
    progress.enable_steady_tick(Duration::from_millis(100));

    let mut zip = ZipArchive::new(File::open(cfpack_path)?)?;

    let mut manifest_string = String::new();
    zip.by_name("manifest.json")?.read_to_string(&mut manifest_string)?;
    let manifest: CfManifest = from_str(&manifest_string)?;

    let cf_mods = CURSEFORGE.get_mods(manifest.files.iter().map(|m| m.project_id).collect()).await?;
    let mut mods: Vec<Mod> = cf_mods.into_iter().map(|m| {
        let version_id = manifest.files.iter().find(|f| f.project_id == m.id).unwrap().file_id;
        Mod {
            name: m.name,
            project_type: m.class_id.unwrap().try_into().unwrap(),
            platform: ModPlatform::CurseForge,
            id: m.id.to_string(),
            version: version_id.to_string(),
            pinned: false,
        }
    }).collect();

    progress.set_message("Extracting overrides");
    zip.extract(env::current_dir()?)?;
    fs::remove_file(env::current_dir()?.join("manifest.json")).ok();
    fs::remove_file(env::current_dir()?.join("modlist.html")).ok();

    let override_mods_dir = env::current_dir()?.join("overrides/mods");
    if override_mods_dir.is_dir() {
        progress.set_message("Attempting to find override mods on modrinth");
        let mut mr_hashes = Vec::new();

        // get fingerprints from files
        for entry in fs::read_dir(&override_mods_dir)? {
            let path = entry?.path();
            if path.is_file() && path.extension().unwrap_or_default() == "jar" {
                let bytes = fs::read(&path)?;
                
                let hash = format!("{:x}", Sha1::digest(&bytes));
                mr_hashes.push(hash);
            }
        }

        let versions = MODRINTH.get_versions_from_hashes(mr_hashes).await?;
        let project_ids: Vec<&str> = versions.iter().map(|v| v.1.project_id.as_str()).collect();
        let projects = MODRINTH.get_multiple_projects(&project_ids).await?;

        for p in projects {
            let version = versions.iter().find(|v| v.1.project_id == p.id).unwrap().1;

            mods.push(Mod {
                name: p.title,
                project_type: p.project_type.try_into().unwrap(),
                platform: ModPlatform::Modrinth,
                id: p.id,
                version: version.id.to_owned(),
                pinned: false,
            });


            let file_path = override_mods_dir.join(primary_file(version.files.to_owned()).filename);
            if file_path.is_file() {
                fs::remove_file(file_path)?;
            }
        }

        if fs::read_dir(&override_mods_dir)?.count() == 0 {
            fs::remove_dir(&override_mods_dir)?;
        }
    }

    let manifest_loader_id = &manifest.minecraft.mod_loaders.iter().find(|l| l.primary).unwrap().id;
    let loader_split: Vec<&str> = manifest_loader_id.split('-').collect();
    let mod_loader = match loader_split[0] {
        "fabric" => ModLoader::Fabric,
        "quilt" => ModLoader::Quilt,
        "forge" => ModLoader::Forge,
        "neoforge" => ModLoader::NeoForge,
        _ => return Err(Error::Other("Pack has unsupported mod loader".into()))
    };
    let loader_version = loader_split[1].to_string();

    let modpack =  Modpack {
        about: ModpackAbout {
            name: manifest.name,
            authors: vec![manifest.author],
            description: None,
            version: manifest.version,
        },
        versions: ModpackVersions {
            minecraft: manifest.minecraft.version,
            mod_loader,
            loader_version,
        }
    };

    Modpack::write(&modpack)?;
    Index::write(&Index { mods, overrides: vec![] })?;

    progress.finish_with_message(format!("Imported {}", modpack.about.name));
    Ok(())
}