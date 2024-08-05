use std::{collections::HashMap, env, fs, hash::Hash, path::PathBuf};

use sha1::{Digest, Sha1};
use sha2::Sha256;
use tokio::try_join;

use crate::{error::{Error, Result}, structs::{index::{AddonSource, Index, ProjectType}, pack::Modpack, packwiz::{CurseforgeModUpdate, DownloadMode, HashFormat, IndexFile, ModDownload, ModUpdate, ModrinthModUpdate, PwIndex, PwIndexInfo, PwMod, PwModHelper, PwPack}}, util::{files::is_local_path, modrinth::primary_file}, CURSEFORGE, MODRINTH};

pub async fn export_packwiz(export_path: PathBuf) -> Result<()> {
    if !export_path.exists() || export_path.read_dir()?.count() != 0 {
        return Err(Error::Other("Please provide an existing and empty folder path to export to".into()));
    }

    let modpack = Modpack::read()?;
    let index = Index::read().await?;

    let mut mr_sources = Vec::new();
    let mut cf_sources = Vec::new();
    let mut gh_sources = Vec::new();

    index.addons.into_iter().for_each(|a| match a.source.clone() {
        AddonSource::Modrinth(source) => mr_sources.push((a, source)),
        AddonSource::Curseforge(source) => cf_sources.push((a, source)),
        AddonSource::Github(source) => gh_sources.push((a, source)),
    });

    let mr_version_ids = mr_sources.iter().map(|a| a.1.version.as_str()).collect::<Vec<&str>>();
    let cf_version_ids = cf_sources.iter().map(|a| (a.1.id, a.1.version)).collect::<Vec<(i32, i32)>>();
    let (mr_versions, cf_files) = try_join!(
        MODRINTH.get_versions(mr_version_ids.as_slice()),
        CURSEFORGE.get_files(cf_version_ids)
    )?;

    let mut pwmods = Vec::new();

    mr_sources.into_iter().for_each(|a| {
        let primary_file = primary_file(mr_versions.iter().find(|v| v.project_id == a.1.id).unwrap().files.clone());
        let pwmod = PwMod {
            name: a.0.name,
            filename: primary_file.filename,
            download: ModDownload {
                url: Some(primary_file.url),
                hash_format: HashFormat::Sha1,
                hash: primary_file.hashes.get("sha1").unwrap().clone(),
                mode: None,
            },
            option: None,
            side: Some(a.0.side),
            update: Some(ModUpdate {
                modrinth: Some(ModrinthModUpdate {
                    mod_id: a.1.id,
                    version: a.1.version,
                }),
                curseforge: None,
            }),
        };
        let pwmod_str = toml::to_string_pretty(&pwmod).unwrap();

        pwmods.push(PwModHelper {
            file_path: a.0.project_type.folder().join(format!("{}.pw.toml", pwmod.name.to_lowercase().replace(' ', "-"))),
            hash: format!("{:x}", Sha256::digest(pwmod_str.as_bytes())),
            pwmod_str,
        });
    });

    cf_sources.into_iter().for_each(|a| {
        let file = cf_files.iter().find(|f| f.mod_id == a.1.id).unwrap().clone();
        let pwmod = PwMod {
            name: a.0.name,
            filename: file.file_name,
            download: ModDownload {
                url: None,
                hash_format: HashFormat::Sha1,
                hash: file.hashes.into_iter().find(|h| matches!(h.algo.into(), HashFormat::Sha1)).unwrap().value,
                mode: Some(DownloadMode::Curseforge),
            },
            option: None,
            side: Some(a.0.side),
            update: Some(ModUpdate {
                modrinth: None,
                curseforge: Some(CurseforgeModUpdate {
                    project_id: a.1.id,
                    file_id: a.1.version,
                }),
            }),
        };
        let pwmod_str = toml::to_string_pretty(&pwmod).unwrap();

        pwmods.push(PwModHelper {
            file_path: a.0.project_type.folder().join(format!("{}.pw.toml", pwmod.name.to_lowercase().replace(' ', "-"))),
            hash: format!("{:x}", Sha256::digest(pwmod_str.as_bytes())),
            pwmod_str,
        });
    });


    
    let pwindex = PwIndex {
        hash_format: HashFormat::Sha256,
        files: pwmods.iter().map(|m| IndexFile {
            file: m.file_path.to_string_lossy().to_string(),
            hash: m.hash.clone(),
            hash_format: None,
            metafile: Some(true),
        }).collect(),
    };
    let pwindex_str = toml::to_string_pretty(&pwindex).unwrap();

    let mut pack_versions: HashMap<String, String> = HashMap::new();
    pack_versions.insert("minecraft".into(), modpack.versions.minecraft);
    pack_versions.insert(modpack.versions.loader.to_string().to_lowercase(), modpack.versions.loader_version);

    let pwpack = PwPack {
        name: modpack.name,
        author: modpack.authors.first().cloned(),
        version: Some(modpack.version),
        description: modpack.description,
        pack_format: "packwiz:1.1.0".into(),
        index: PwIndexInfo {
            file: "index.toml".into(),
            hash_format: HashFormat::Sha256,
            hash: format!("{:x}", Sha256::digest(pwindex_str.as_bytes())),
        },
        versions: pack_versions,
    };
    let pwpack_str = toml::to_string_pretty(&pwpack).unwrap();

    fs::write(export_path.join("pack.toml"), pwpack_str)?;
    fs::write(export_path.join("index.toml"), pwindex_str)?;

    for file in pwmods {
        let full_path = export_path.join(file.file_path);
        let parent_dir = full_path.parent().unwrap();
        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir)?;
        }
        fs::write(full_path, file.pwmod_str)?;
    }
    
    Ok(())
}

impl ProjectType {
    fn folder(&self) -> PathBuf {
        match self {
            ProjectType::Mod => "mods",
            ProjectType::Shader => "shaderpacks",
            ProjectType::Datapack => "datapacks",
            ProjectType::Resourcepack => "resourcepacks",
            _ => "overrides/unknown"
        }.into()
    }
}

impl From<i32> for HashFormat {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::Sha1,
            2 => Self::Md5,
            _ => unreachable!()
        }
    }
}