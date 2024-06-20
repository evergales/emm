use std::{env, fs::{self, File}, io::Read, path::PathBuf, time::Duration};

use dialoguer::Confirm;
use indicatif::ProgressBar;
use serde_json::from_str;
use zip::ZipArchive;

use crate::{commands::export::{Metadata, PackDependency}, structs::{Index, Mod, ModLoader, Modpack, ModpackAbout, ModpackVersions}, Error, Result, MODRINTH};

pub async fn import_modrinth(mrpack_path: PathBuf) -> Result<()> {
    if Modpack::read().is_ok() {
        let confirm = Confirm::new()
            .with_prompt("Importing will overwrite your current modpack, continue?")
            .interact()?;

        if !confirm {
            return Ok(());
        }
    }

    if !mrpack_path.is_file() || mrpack_path.extension().unwrap_or_default() != "mrpack" {
        return Err(Error::Other("The path you provided is not an mrpack file".to_string()));
    }

    let progress = ProgressBar::new_spinner().with_message("Reading mrpack file");
    progress.enable_steady_tick(Duration::from_millis(100));

    let mut zip = ZipArchive::new(File::open(mrpack_path)?)?;

    let mut mrpack_string = String::new();
    zip.by_name("modrinth.index.json")?
        .read_to_string(&mut mrpack_string)?;
    let mrpack: Metadata = from_str(&mrpack_string)?;

    let mc_version = mrpack.dependencies.get_key_value(&PackDependency::Minecraft).unwrap().1.to_owned();
    let (mod_loader, loader_version) = mrpack.dependencies.into_iter().find(|v| v.0 != PackDependency::Minecraft).unwrap();

    let modpack = Modpack {
        about: ModpackAbout {
            name: mrpack.name,
            authors: vec!["you!".to_string()],
            description: mrpack.summary,
            version: mrpack.version_id,
        },
        versions: ModpackVersions {
            minecraft: mc_version,
            mod_loader: mod_loader.try_into()?,
            loader_version,
        },
    };

    Modpack::write(&modpack)?;

    progress.set_message("Adding mods");

    // find projects from the hashes provided in mrpack files
    // order doesnt stay and Project doesnt include file hashes so searching is needed later
    let file_hashes: Vec<String> = mrpack.files.into_iter().map(|f| f.hashes.sha1).collect();
    let versions = MODRINTH.get_versions_from_hashes(file_hashes).await?;
    let project_ids: Vec<&str> = versions.iter().map(|v| v.1.project_id.as_str()).collect();
    let projects = MODRINTH.get_multiple_projects(&project_ids).await?;

    let mut mods: Vec<Mod> = projects.into_iter().map(|project| {
        let version_hash = versions.iter().find(|v| v.1.project_id == project.id).unwrap().0;
        Mod {
            name: project.title,
            modrinth_id: Some(project.id),
            curseforge_id: None,
            version: version_hash.to_owned(),
            pinned: false,
        }
    }).collect();

    mods.sort_by_key(|m| m.name.to_owned());
    Index::write(&Index { mods })?;

    // afaik zip doesnt have a way to extract certain directories (that isnt way too tedious)
    progress.set_message("Extracting overrides");
    zip.extract(env::current_dir()?)?;
    fs::remove_file(env::current_dir()?.join("modrinth.index.json"))?;

    progress.finish_and_clear();
    Ok(())
}

impl TryFrom<PackDependency> for ModLoader {
    type Error = crate::Error;
    fn try_from(value: PackDependency) -> std::result::Result<Self, Self::Error> {
        match value {
            PackDependency::Forge => Ok(Self::Forge),
            PackDependency::NeoForge => Ok(Self::NeoForge),
            PackDependency::FabricLoader => Ok(Self::Fabric),
            PackDependency::QuiltLoader => Ok(Self::Quilt),
            PackDependency::Minecraft => Err(Error::Parse("Tried to parse 'Minecraft' mrpack DependencyType into ModLoader".to_string())),
        }
    }
}