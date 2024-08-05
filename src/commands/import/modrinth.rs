use std::{env, fs::{self, File}, io::Read, path::PathBuf, time::Duration};

use indicatif::ProgressBar;
use zip::ZipArchive;

use crate::{api::curseforge::{CurseAPI, File as CurseFile}, error::{Error, Result}, structs::{index::{Addon, AddonOptions, AddonSource, CurseforgeSource, Index, ModrinthSource, Side}, mrpack::{Metadata, PackDependency}, pack::{ModLoader, Modpack, PackOptions, Versions}}, util::modrinth::get_side, CURSEFORGE, MODRINTH};

pub async fn import_modrinth(mrpack_path: PathBuf) -> Result<()> {
    if !mrpack_path.is_file() || mrpack_path.extension().unwrap_or_default() != "mrpack" {
        return Err(Error::Other("The path you provided is not an mrpack file".into()));
    }

    let progress = ProgressBar::new_spinner().with_message("Reading mrpack file");
    progress.enable_steady_tick(Duration::from_millis(100));

    let mut zip = ZipArchive::new(File::open(mrpack_path)?)?;

    let mut mrpack_string = String::new();
    zip.by_name("modrinth.index.json")?.read_to_string(&mut mrpack_string)?;
    let mrpack: Metadata = serde_json::from_str(&mrpack_string)?;

    let mc_version = mrpack.dependencies.get(&PackDependency::Minecraft).unwrap().clone();
    let (mod_loader, loader_version) = mrpack.dependencies.into_iter().find(|v| v.0 != PackDependency::Minecraft).unwrap();

    let modpack = Modpack {
        name: mrpack.name,
        version: mrpack.version_id,
        authors: vec![],
        description: mrpack.summary,
        index_path: "./index".into(),
        options: PackOptions::default(),
        versions: Versions {
            minecraft: mc_version,
            loader: mod_loader.try_into()?,
            loader_version,
        },
    };

    Modpack::write(&modpack)?;

    progress.set_message("Adding mods");

    // find projects from the hashes provided in mrpack files
    // order doesnt stay and Project doesnt include file hashes so searching is needed later
    let file_hashes: Vec<&str> = mrpack.files.iter().map(|f| f.hashes.sha1.as_str()).collect();
    let versions = MODRINTH.versions_from_hashes(&file_hashes).await?;
    let project_ids: Vec<&str> = versions.iter().map(|v| v.1.project_id.as_str()).collect();
    let projects = MODRINTH.get_multiple_projects(&project_ids).await?;

    let mut addons: Vec<Addon> = projects.into_iter().map(|project| {
        let version = versions.iter().find(|v| v.1.project_id == project.id).unwrap();
        Addon {
            name: project.title,
            project_type: project.project_type,
            side: get_side(&project.client_side, &project.server_side),
            source: AddonSource::Modrinth(ModrinthSource {
                id: project.id,
                version: version.1.id.clone(),
            }),
            options: Some(AddonOptions::default()),
            filename: None,
            
        }
    }).collect();

    // afaik zip doesnt have a way to extract only certain directories (that isnt way too tedious)
    progress.set_message("Extracting overrides");
    zip.extract(env::current_dir()?)?;
    fs::remove_file(env::current_dir()?.join("modrinth.index.json"))?;

    let override_mods_dir = env::current_dir()?.join("overrides/mods");
    if override_mods_dir.is_dir() {
        progress.set_message("Attempting to find override mods on curseforge");
        let mut cf_fingerprints: Vec<u32> = Vec::new();

        // get fingerprints from files
        for entry in fs::read_dir(&override_mods_dir)? {
            let path = entry?.path();
            if path.is_file() && path.extension().unwrap_or_default() == "jar" {
                let bytes = fs::read(&path)?;
                cf_fingerprints.push(CurseAPI::get_cf_fingerprint(&bytes));
            }
        }

        // find fingerprint matches
        let matches = CURSEFORGE.get_fingerprint_matches(&cf_fingerprints).await?;
        let cf_files: Vec<CurseFile> = matches.exact_matches.into_iter().map(|m| m.file).collect();
        let cf_addons = CURSEFORGE.get_mods(cf_files.iter().map(|f| f.mod_id).collect()).await?;

        for addon in cf_addons {
            let version_file = cf_files.iter().find(|f| f.mod_id == addon.id).unwrap();

            addons.push(Addon {
                name: addon.name,
                project_type: addon.class_id.unwrap().try_into()?,
                side: Side::Both,
                source: AddonSource::Curseforge(CurseforgeSource {
                    id: addon.id,
                    version: version_file.id
                }),
                options: Some(AddonOptions::default()),
                filename: None
            });

            let file_path = override_mods_dir.join(version_file.file_name.clone());
            if file_path.is_file() {
                fs::remove_file(file_path)?;
            }
        }

        if fs::read_dir(&override_mods_dir)?.count() == 0 {
            fs::remove_dir(&override_mods_dir)?;
        } 
    }

    Index::write_addons(addons).await?;
    progress.finish_with_message(format!("Imported {}", modpack.name));
    Ok(())
}

impl TryFrom<PackDependency> for ModLoader {
    type Error = crate::error::Error;
    fn try_from(value: PackDependency) -> std::result::Result<Self, Self::Error> {
        match value {
            PackDependency::Forge => Ok(Self::Forge),
            PackDependency::NeoForge => Ok(Self::NeoForge),
            PackDependency::FabricLoader => Ok(Self::Fabric),
            PackDependency::QuiltLoader => Ok(Self::Quilt),
            PackDependency::Minecraft => Err(Error::Other("Tried to parse 'Minecraft' mrpack DependencyType into ModLoader".into())),
        }
    }
}