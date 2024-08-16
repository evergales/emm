use std::{collections::HashMap, env, fs, io::Write, path::{Path, PathBuf}, sync::Arc};

use tokio::{sync::Semaphore, task::JoinSet};
use zip::{write::SimpleFileOptions, ZipWriter};

use crate::{
    api::modrinth::VersionFile, cli::ExportModrinthArgs, error::Result, structs::{
        index::{AddonSource, Index,  ProjectType}, mrpack::{File, FileHashes, Game, Metadata, PackDependency}, pack::Modpack
    }, util::{files::{add_recursively, download_file}, modrinth::primary_file}, CURSEFORGE, GITHUB, MODRINTH
};

pub async fn export_modrinth(args: ExportModrinthArgs) -> Result<()> {
    let modpack = Arc::new(Modpack::read()?);
    let index = Index::read().await?;

    // Vec<(Source, ProjectType)>
    let mut mr_addons = Vec::new();
    let mut cf_addons = Vec::new();
    let mut gh_addons = Vec::new();

    index.addons.into_iter().for_each(|a| match a.source {
        AddonSource::Modrinth(source) => mr_addons.push((source, a.project_type)),
        AddonSource::Curseforge(source) => cf_addons.push((source, a.project_type)),
        AddonSource::Github(source) => gh_addons.push((source, a.project_type))
    });

    let mr_versions = MODRINTH.get_versions(
        mr_addons
            .iter()
            .map(|a| a.0.version.as_str())
            .collect::<Vec<&str>>()
            .as_slice(),
    ).await?;

    let mr_files: Vec<(&ProjectType, VersionFile)> =  mr_versions.into_iter().map(|v| {
        let project_type = &mr_addons.iter().find(|a| a.0.id == v.project_id).unwrap().1;
        let file = primary_file(v.files);
        (project_type, file)
    }).collect();

    // map files into mrpack files
    let files: Vec<File> = mr_files.into_iter().map(|f| {
        let folder = f.0.export_folder(modpack.options.clone());

        File {
            path: format!("{}/{}", folder.to_string_lossy(), f.1.filename).into(),
            hashes: FileHashes {
                sha1: f.1.hashes.get("sha1").unwrap().clone(),
                sha512: f.1.hashes.get("sha512").unwrap().clone(),
            },
            env: None,
            downloads: vec![f.1.url],
            file_size: f.1.size,
        }
    })
    .collect();

    let mut pack_dependencies: HashMap<PackDependency, String> = HashMap::new();
    pack_dependencies.insert(PackDependency::Minecraft, modpack.versions.minecraft.clone());
    pack_dependencies.insert(modpack.versions.loader.clone().into(), modpack.versions.loader_version.clone());

    let metadata = Metadata {
        format_version: 1,
        game: Game::Minecraft,
        version_id: modpack.version.clone(),
        name: modpack.name.clone(),
        summary: modpack.description.clone(),
        files,
        dependencies: pack_dependencies,
    };

    let cache_dir = env::temp_dir().join(format!("emm-export-cache-{}", std::process::id()));
    let mod_overrides = if !cf_addons.is_empty() || !gh_addons.is_empty() { Some(&cache_dir) } else { None };
    if !cf_addons.is_empty() || !gh_addons.is_empty() {
        fs::create_dir(&cache_dir)?;
        // (file_path, download_url)
        let mut to_download: Vec<(PathBuf, String)> = Vec::new();

        if !cf_addons.is_empty() {
            let cf_files = CURSEFORGE.get_files(cf_addons.iter().map(|a| (a.0.id, a.0.version)).collect()).await?;
    
            to_download.extend(cf_files.into_iter().map(|f| {
                let project_type = &cf_addons.iter().find(|a| a.0.id == f.mod_id).unwrap().1;
                let file_path = project_type.export_folder(modpack.options.clone()).join(f.file_name);
                (file_path, f.download_url.unwrap())
            }));
        }

        if !gh_addons.is_empty() {            
            let mut tasks: JoinSet<Result<(PathBuf, String)>> = JoinSet::new();
            for addon in gh_addons {
                let modpack = modpack.clone();
                
                let task = async move {
                    let repo_split: Vec<&str> = addon.0.repo.split('/').collect();
                    let release = GITHUB.get_release_by_tag(repo_split[0], repo_split[1], &addon.0.tag).await?;
                    let asset = &release.assets[addon.0.asset_index];
                    let file_path = addon.1.export_folder(modpack.options.clone()).join(asset.name.clone());
    
                    Ok((file_path, asset.browser_download_url.clone()))
                };
    
                tasks.spawn(task);
            }

            while let Some(res) = tasks.join_next().await { to_download.push(res??) };
        }

        let permits = Arc::new(Semaphore::new(10)); // limit file downloads to 10 at a time
        let mut tasks: JoinSet<Result<()>> = JoinSet::new();
        for file in to_download {
            let permits = permits.clone();
            let cache_dir = cache_dir.clone();
            let parent_folder = file.0.parent().unwrap();
            if !&cache_dir.join(parent_folder).is_dir() {
                fs::create_dir(&cache_dir.join(parent_folder))?;
            }

            let task = async move {
                let _permit = permits.acquire().await.unwrap();
                download_file(&cache_dir.join(&file.0), &file.1).await?;
                Ok(())
            };

            tasks.spawn(task);
        }

        while let Some(res) = tasks.join_next().await { res?? };
    }

    create_mrpack(&env::current_dir()?, &metadata, args.overrides_path.as_ref(), mod_overrides).unwrap();
    if cache_dir.is_dir() {
        fs::remove_dir_all(cache_dir)?;
    }

    Ok(())
}

fn create_mrpack(path: &Path, metadata: &Metadata, overrides: Option<&PathBuf>, mod_overrides: Option<&PathBuf>) -> zip::result::ZipResult<()> {
    let zip_path = path.join(format!("{}-{}.mrpack", metadata.name, metadata.version_id));
    let mut zip = ZipWriter::new(fs::File::create(zip_path).unwrap());
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("modrinth.index.json", options)?;
    let metadata_str = serde_json::to_string_pretty(&metadata).unwrap();
    zip.write_all(metadata_str.as_bytes())?;

    if overrides.is_some() || mod_overrides.is_some() {
        zip.add_directory("overrides", options)?;
    }

    if overrides.is_some() {
        add_recursively(overrides.unwrap(), &PathBuf::from("overrides"), &mut zip, options)?;
    }

    if mod_overrides.is_some() {
        add_recursively(mod_overrides.unwrap(), &PathBuf::from("overrides"), &mut zip, options)?;
    }

    zip.finish()?;
    Ok(())
}
