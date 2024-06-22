use std::{collections::HashMap, env, fs::{self, File}, io::{Read, Write}, path::PathBuf, sync::Arc, time::Duration};

use ferinth::structures::version::VersionFile;
use indicatif::ProgressBar;
use tokio::{sync::Semaphore, task::JoinSet};
use walkdir::WalkDir;
use zip::{write::SimpleFileOptions, ZipWriter};

use crate::{structs::{Index, Mod, ModLoader, ModPlatform, Modpack}, util::{join_all, primary_file, seperate_mods_by_platform}, Result, CURSEFORGE, MODRINTH};

use super::{FileHashes, Game, Metadata, PackDependency};

pub async fn export_modrinth(overrides_path: Option<PathBuf>) -> Result<()> {
    let modpack = Modpack::read()?;
    let index = Index::read()?;
    let (mr_mods, cf_mods) = seperate_mods_by_platform(index.mods.clone()).await?;

    let progress = ProgressBar::new_spinner().with_message("Starting export");
    progress.enable_steady_tick(Duration::from_millis(100));

    // get primary files from stored versions from modrinth
    let mr_verions = MODRINTH
        .get_multiple_versions(
            mr_mods
                .iter()
                .map(|m| m.version.as_str())
                .collect::<Vec<&str>>()
                .as_slice(),
        )
        .await?;
    let mr_files: Vec<VersionFile> =  mr_verions.into_iter().map(|v| primary_file(v.files)).collect();

    // map files into mrpack files
    let files = mr_files.into_iter().map(|f| super::File {
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
        modpack.versions.mod_loader.into(),
        modpack.versions.loader_version,
    );

    let metadata = Metadata {
        format_version: 1,
        game: Game::Minecraft,
        version_id: modpack.about.version,
        name: modpack.about.name,
        summary: modpack.about.description,
        files,
        dependencies: pack_dependencies,
    };

    /* 
    todo:
    currently all mods & other downloadable cf projects get dumped into the mods folder
    these should all go into their own individual folders
    */

    let mod_overrides: Option<PathBuf>;

    let cache_dir = env::temp_dir().join(format!("emm-export-cache-{}", std::process::id()));
    if !cf_mods.is_empty() {
        progress.set_message("Downloading curseforge mods");
        fs::create_dir(&cache_dir)?;
    
        let files = CURSEFORGE.get_files(cf_mods.into_iter().map(|m| m.version).collect::<Vec<i32>>()).await?;
        let permits = Arc::new(Semaphore::new(10)); // limit file downloads to 10 at a time
        let index_cf_mods: Vec<Mod> = index.mods.into_iter().filter(|m| matches!(m.platform, ModPlatform::CurseForge)).collect();

        let mut tasks: JoinSet<crate::Result<()>> = JoinSet::new();
        for file in files {
            let cache_dir = cache_dir.clone();
            let permits = permits.clone();
            let project_type = &index_cf_mods.iter().find(|m| m.id == file.mod_id.to_string()).unwrap().project_type;
            let folder_name = format!("{}s", project_type);
            if !&cache_dir.join(&folder_name).is_dir() {
                fs::create_dir(&cache_dir.join(&folder_name))?;
            }

            let task = async move {
                let _permit = permits.acquire().await.unwrap();
                download_file(&cache_dir.join(folder_name).join(file.file_name), &file.download_url.unwrap().to_string()).await?;
                Ok(())
            };
            tasks.spawn(task);
        }

        join_all(tasks).await?;
        mod_overrides = Some(cache_dir.clone());
    } else {
        mod_overrides = None
    }

    progress.set_message("Creating mrpack file");
    create_mrpack(env::current_dir()?, metadata, overrides_path, mod_overrides).unwrap();
    if cache_dir.is_dir() {
        progress.set_message("Cleaning up");
        fs::remove_dir_all(cache_dir)?;
    }

    progress.finish_with_message("Exported to mrpack!");
    Ok(())
}

pub async fn download_file(path: &PathBuf, url: &String) -> Result<()> {
    let res = reqwest::get(url).await?;
    let data = &*res.bytes().await?;
    let mut file = File::create(path)?;
    file.write_all(data)?;
    Ok(())
}

pub fn create_mrpack(path: PathBuf, metadata: Metadata, overrides: Option<PathBuf>, mod_overrides: Option<PathBuf>) -> zip::result::ZipResult<()> {
    let zip_path: PathBuf = path.join(format!("{}-{}.mrpack", metadata.name, metadata.version_id));
    let mut zip = ZipWriter::new(File::create(zip_path).unwrap());
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(
        "modrinth.index.json",
        options,
    )?;
    let metadata_str = serde_json::to_string_pretty(&metadata).unwrap();
    zip.write_all(metadata_str.as_bytes())?;

    if overrides.is_some() || mod_overrides.is_some() {
        zip.add_directory("overrides", options)?;
    }

    if overrides.is_some() {
        add_recursively(overrides.unwrap(), "overrides".into(), &mut zip, options)?;
    }

    if mod_overrides.is_some() {
        add_recursively(mod_overrides.unwrap(), "overrides".into(), &mut zip, options)?;
    }

    zip.finish()?;
    Ok(())
}

// https://github.com/zip-rs/zip2/blob/master/examples/write_dir.rs
fn add_recursively(from_path: PathBuf, zip_path: PathBuf, zip: &mut ZipWriter<File>, options: SimpleFileOptions) -> zip::result::ZipResult<()> {
    let mut buffer = Vec::new();
    for entry in WalkDir::new(&from_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let file_name = path.strip_prefix(&from_path).unwrap();
        let path_as_string = file_name.to_str().to_owned().unwrap();

        if path.is_file() {
            zip.start_file_from_path(zip_path.join(path_as_string), options)?;
            let mut f = File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
            buffer.clear()
        }
    }

    Ok(())
}

impl From<ModLoader> for PackDependency {
    fn from(value: ModLoader) -> Self {
        match value {
            ModLoader::Fabric => Self::FabricLoader,
            ModLoader::Quilt => Self::QuiltLoader,
            ModLoader::Forge => Self::Forge,
            ModLoader::NeoForge => Self::NeoForge,
        }
    }
}