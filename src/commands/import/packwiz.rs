use std::{fs, path::PathBuf};

use serde::de::DeserializeOwned;
use tokio::task::JoinSet;

use crate::{cli::ImportPackwizArgs, error::{Error, Result}, structs::{index::{Addon, AddonOptions, AddonSource, CurseforgeSource, Index, ModrinthSource, ProjectType}, pack::{ModLoader, Modpack, PackOptions, Versions}, packwiz::{IndexFile, ModUpdate, PwIndex, PwMod, PwPack}}};

pub async fn import_packwiz(args: ImportPackwizArgs) -> Result<()> {
    if !args.source.ends_with("pack.toml") {
        return Err(Error::BadImport("please provide a valid url or path to a packwiz pack.toml file".into()));
    }
    if !args.source.starts_with("http") {
        let source_path = PathBuf::from(&args.source);
        if !source_path.exists() {
            return Err(Error::BadImport("the path you provided does not exist".into()));
        }
    }

    let base_path = args.source.strip_suffix("/pack.toml").unwrap().to_string();

    let source_pack: PwPack = PwFile::from(&args.source).get_content().await?;
    let file_index: PwIndex = PwFile::from(format!("{}/{}", base_path, source_pack.index.file)).get_content().await?;

    let mut tasks: JoinSet<Result<Addon>> = JoinSet::new();
    for file in file_index.files {
        let base_path = base_path.clone();
        let task = async move {
            let pw_mod: PwMod = PwFile::from(format!("{}/{}", base_path, file.file)).get_content().await?;
            Ok(Addon {
                name: pw_mod.name.clone(),
                project_type: file.get_project_type(),
                side: pw_mod.side.unwrap_or_default(),
                source: pw_mod.update.try_into().map_err(|err: Error| Error::Other(format!("{}: {}", pw_mod.name, err)))?,
                options: Some(AddonOptions::default()),
                filename: None,
            })
        };

        tasks.spawn(task);
    }

    let mut addons = Vec::new();
    while let Some(res) = tasks.join_next().await {
        addons.push(res??);
    };

    let mc_version = source_pack.versions.get("minecraft").unwrap();
    let (loader, loader_version) = {
        match source_pack.versions.iter().find(|(key, _)| *key != "minecraft") {
            Some(res) => Ok(res),
            None => Err(Error::BadImport("modpack does not have a mod loader".into())),
        }
    }?;


    let modpack = Modpack {
        name: source_pack.name,
        version: source_pack.version.unwrap_or("0.1.0".into()),
        authors: vec![source_pack.author.unwrap_or_default()],
        description: source_pack.description,
        index_path: "./index".into(),
        options: PackOptions::default(),
        versions: Versions {
            minecraft: mc_version.clone(),
            loader: loader.into(),
            loader_version: loader_version.clone(),
        },
    };

    Modpack::write(&modpack)?;
    Index::write_addons(addons).await?;
    println!("Imported {}", modpack.name);
    Ok(())
}

enum PwFile {
    Local(PathBuf),
    Url(String)
}

impl PwFile {
    async fn get_content<T: DeserializeOwned>(&self) -> Result<T> {
        let str = match self {
            PwFile::Local(path) => {
                fs::read_to_string(path)?
            },
            PwFile::Url(url) => {
                reqwest::get(url)
                    .await?
                    .error_for_status()?
                    .text()
                    .await?
            },
        };
        Ok(toml::from_str(&str).unwrap())
    }
}

impl<T: AsRef<str>> From<T> for PwFile {
    fn from(value: T) -> Self {
        let value = value.as_ref();
        if value.starts_with("http") {
            Self::Url(value.into())
        } else {
            Self::Local(value.into())
        }
    }
}

impl TryFrom<Option<ModUpdate>> for AddonSource {
    type Error = Error;
    fn try_from(value: Option<ModUpdate>) -> Result<Self> {
        match value {
            Some(value) => {
                if let Some(source) = value.modrinth {
                    Ok(Self::Modrinth(ModrinthSource {
                        id: source.mod_id,
                        version: source.version,
                    }))
                } else if let Some(source) = value.curseforge {
                    Ok(Self::Curseforge(CurseforgeSource {
                        id: source.file_id,
                        version: source.project_id,
                    }))
                } else {
                    Err(Error::Other("unable to import mod as it does not have update information".into()))
                }
            },
            None => Err(Error::Other("unable to import mod as it does not have update information".into())),
        }
        
    }
}

impl IndexFile {
    fn get_project_type(&self) -> ProjectType {
        let type_str = self.file.split_once('/').unwrap().0;
        match type_str {
            "mods" => ProjectType::Mod,
            "resourcepacks" => ProjectType::Resourcepack,
            "datapacks" => ProjectType::Datapack,
            "shaders" => ProjectType::Shader,
            _ => ProjectType::Unknown
        }
    }
}

impl<T: AsRef<str>> From<T> for ModLoader {
    fn from(value: T) -> Self {
        match value.as_ref() {
            "fabric" => Self::Fabric,
            "quilt" => Self::Quilt,
            "forge" => Self::Forge,
            "neoforge" => Self::NeoForge,
            _ => unreachable!()
        }
    }
}