use std::{collections::HashMap, env, fs::{self, File}, path::PathBuf};

use ferinth::structures::version::VersionFile;
use mrpack::structs::{FileHashes, Metadata, PackDependency};
use tokio::task::JoinSet;

use crate::{
    structs::{Index, ModLoader, Modpack}, util::{join_all, seperate_mods_by_platform}, Result, CURSEFORGE, MODRINTH
};

pub async fn export_modrinth(overrides_path: Option<PathBuf>) -> Result<()> {
    let modpack = Modpack::read()?;
    let index = Index::read()?;
    let (mr_mods, cf_mods) = seperate_mods_by_platform(index.mods).await?;

    // get primary files from stored version hashes from modrinth
    let mr_files: Vec<VersionFile> = MODRINTH
        .get_versions_from_hashes(
            mr_mods
                .into_iter()
                .map(|m| m.version)
                .collect::<Vec<String>>(),
        )
        .await?
        .into_iter()
        .map(|v| primary_file(v.1.files))
        .collect();

    // map files into mrpack files
    let files = mr_files.into_iter().map(|f| mrpack::structs::File {
        path: format!("mods/{}", f.filename).into(),
        hashes: FileHashes {
            sha1: f.hashes.sha1,
            sha512: f.hashes.sha512,
        },
        env: None,
        downloads: vec![f.url],
        file_size: f.size,
    })
    .collect();

    let mut pack_dependencies: HashMap<PackDependency, String> = HashMap::new();
    pack_dependencies.insert(PackDependency::Minecraft, modpack.versions.minecraft);
    pack_dependencies.insert(
        modpack.versions.mod_loader.into_packdependency(),
        modpack.versions.loader_version,
    );

    let metadata = Metadata {
        format_version: 1,
        game: mrpack::structs::Game::Minecraft,
        version_id: modpack.about.version,
        name: modpack.about.name,
        summary: modpack.about.description,
        files,
        dependencies: pack_dependencies,
    };

    let mod_overrides: Option<PathBuf>;

    let cache_dir = env::current_dir()?.join(".cache");
    if !cf_mods.is_empty() {
        fs::create_dir_all(&cache_dir)?;
    
        let files = CURSEFORGE.get_files(cf_mods.into_iter().map(|m| m.version).collect::<Vec<i32>>()).await?;

        let mut tasks: JoinSet<crate::Result<()>> = JoinSet::new();
        for file in files {
            let cache_dir = cache_dir.clone();
            let task = async move {
                download_file(&cache_dir.join(file.file_name), &file.download_url.unwrap().to_string()).await?;
                Ok(())
            };
            tasks.spawn(task);
        }

        join_all(tasks).await?;
        mod_overrides = Some(cache_dir.clone());
    } else {
        mod_overrides = None
    }

    mrpack::create(env::current_dir()?, metadata, overrides_path, mod_overrides).unwrap();
    fs::remove_dir_all(cache_dir)?;
    Ok(())
}

fn primary_file(files: Vec<VersionFile>) -> VersionFile {
    files.into_iter().find(|f| f.primary).unwrap()
}

pub async fn download_file(path: &PathBuf, url: &String) -> Result<()> {
    let res = reqwest::get(url).await?;
    let body = res.text().await?;
    let mut out = File::create(path)?;
    std::io::copy(&mut body.as_bytes(), &mut out)?;
    Ok(())
}

impl ModLoader {
    fn into_packdependency(self) -> PackDependency {
        match self {
            ModLoader::Fabric => PackDependency::FabricLoader,
            ModLoader::Quilt => PackDependency::QuiltLoader,
            ModLoader::Forge => PackDependency::Forge,
            ModLoader::NeoForge => PackDependency::NeoForge,
        }
    }
}