use std::{env, fs::{self, File}, io::Write, path::PathBuf, sync::Arc, time::Duration};

use ferinth::structures::version::VersionFile;
use indicatif::ProgressBar;
use tokio::{sync::Semaphore, task::JoinSet};
use zip::{write::SimpleFileOptions, ZipWriter};

use crate::{structs::{Index, Modpack}, util::{add_recursively, download_file, join_all, primary_file, seperate_mods_by_platform}, Result, CURSEFORGE, MODRINTH};

use super::{CfFile, CfManifest, CfMinecraft, CfModLoader};

pub async fn export_curseforge(overrides_path: Option<PathBuf>) -> Result<()> {
    let modpack = Modpack::read()?;
    let index = Index::read()?;
    let (mr_mods, cf_mods) = seperate_mods_by_platform(index.mods.clone()).await?;

    let progress = ProgressBar::new_spinner().with_message("Starting export");
    progress.enable_steady_tick(Duration::from_millis(100));

    let files: Vec<CfFile> = cf_mods.iter().map(|m| CfFile {
        project_id: m.id,
        file_id: m.version,
        required: true,
    }).collect();

    let manifest = CfManifest {
        minecraft: CfMinecraft {
            version: modpack.versions.minecraft,
            mod_loaders: vec![CfModLoader {
                id: format!("{}-{}", modpack.versions.mod_loader.to_string().to_lowercase(), modpack.versions.loader_version),
                primary: true
            }],
        },
        manifest_type: "minecraftModpack".into(),
        manifest_version: 1,
        name: modpack.about.name,
        version: modpack.about.version,
        author: modpack.about.authors.join(", "),
        files,
        overrides: "overrides".into(),
    };

    let mut modlist = String::new();
    modlist.push_str("<ul>");

    let modlist_mods = if !cf_mods.is_empty() { 
        CURSEFORGE.get_mods(cf_mods.iter().map(|m| m.id).collect()).await?
    } else { Vec::new() };

    for m in modlist_mods {
        modlist.push_str(&format!("\n<li><a href=\"{}\">{}</a></li>", m.links.website_url, m.name))
    }

    modlist.push_str("\n</ul>");

    let cache_dir = env::temp_dir().join(format!("emm-export-cache-{}", std::process::id()));
    let mod_overrides: Option<PathBuf>;
    if !mr_mods.is_empty() {
        progress.set_message("Downloading modrinth mods");
        fs::create_dir(&cache_dir)?;

        let versions = MODRINTH
            .get_multiple_versions(
                &mr_mods
                    .iter()
                    .map(|m| m.version.as_str())
                    .collect::<Vec<&str>>(),
            )
            .await?;
        let files: Vec<(VersionFile, String)> = versions
            .iter()
            .map(|v| primary_file(v.files.to_owned()))
            .zip(versions.iter().map(|v| v.project_id.to_owned()))
            .collect();
        
        let permits = Arc::new(Semaphore::new(10)); // limit file downloads to 10 at a time

        let mut tasks: JoinSet<crate::Result<()>> = JoinSet::new();
        for file in files {
            let cache_dir = cache_dir.clone();
            let permits = permits.clone();
            let project_type = &mr_mods.iter().find(|m| m.id == file.1).unwrap().project_type;
            let folder_name = format!("{}s", project_type);
            if !&cache_dir.join(&folder_name).is_dir() {
                fs::create_dir(&cache_dir.join(&folder_name))?;
            }

            let task = async move {
                let _permit = permits.acquire().await.unwrap();
                download_file(&cache_dir.join(folder_name).join(file.0.filename), &file.0.url.to_string()).await?;
                Ok(())
            };
            tasks.spawn(task);
        }

        join_all(tasks).await?;
        mod_overrides = Some(cache_dir.clone());
    } else {
        mod_overrides = None
    }

    progress.set_message("Creating curseforge pack file");
    create_cfpack(env::current_dir()?, manifest, modlist, overrides_path, mod_overrides)?;
    if cache_dir.is_dir() {
        progress.set_message("Cleaning up");
        fs::remove_dir_all(cache_dir)?;
    }

    progress.finish_with_message("Exported to curseforge pack!");
    Ok(())
}

fn create_cfpack(path: PathBuf, manifest: CfManifest, modlist: String, overrides: Option<PathBuf>, mod_overrides: Option<PathBuf>) -> zip::result::ZipResult<()> {
    let zip_path: PathBuf = path.join(format!("{}-{}.zip", manifest.name, manifest.version));
    let mut zip = ZipWriter::new(File::create(zip_path).unwrap());
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("manifest.json", options)?;
    let manifest_str = serde_json::to_string_pretty(&manifest).unwrap();
    zip.write_all(manifest_str.as_bytes())?;

    zip.start_file("modlist.html", options)?;
    zip.write_all(modlist.as_bytes())?;

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