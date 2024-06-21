use std::{env, fmt::Display, fs, path::PathBuf};
use ferinth::structures::project::ProjectType as MRProjectType;
use serde::{Deserialize, Serialize};
use url::Url;
use crate::{Error, Result};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Modpack {
    pub about: ModpackAbout,
    pub versions: ModpackVersions,
}

impl Modpack {
    pub fn new(name: String, authors: Vec<String>, description: Option<String>, version: String, minecraft_version: String, mod_loader: ModLoader, loader_version: String) -> Self {
        Modpack { about: ModpackAbout { name, authors, description, version }, versions: ModpackVersions { minecraft: minecraft_version, mod_loader, loader_version } }
    }

    fn path() -> PathBuf {
        env::current_dir().unwrap().join("mcpack.toml")
    }

    pub fn read() -> Result<Self> {
        if !Self::path().is_file() { return Err(Error::Uninitialized); }
        let packfile_str = &fs::read_to_string(Self::path()).map_err(|_| Error::Uninitialized)?;
        let toml_str = toml::from_str(packfile_str).map_err(|err| Error::Parse(format!("Error while parsing config to toml {err}")))?;
        Ok(toml_str)
    }

    pub fn write(modpack: &Self) -> Result<()> {
        let str = toml::to_string(modpack).unwrap();
        fs::write(Self::path(), str)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModpackAbout {
    pub name: String,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub version: String
    // maybe resource links here?
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModpackVersions {
    pub minecraft: String,
    pub mod_loader: ModLoader,
    pub loader_version: String
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Index {
    pub mods: Vec<Mod>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<Url>
}

impl Index {
    fn path() -> PathBuf {
        env::current_dir().unwrap().join("index.toml")
    }

    pub fn read() -> Result<Self> {
        if !Modpack::path().is_file() {
            return Err(Error::Uninitialized);
        }
        if !Self::path().is_file() {
            Self::write(&Index { mods: Vec::new(), overrides: Vec::new() })?;
        }
        let index_str = &fs::read_to_string(Self::path()).map_err(|_| Error::Uninitialized)?;
        let toml_str = toml::from_str(index_str).map_err(|err| Error::Parse(format!("Error while parsing index to toml {err}")))?;
        Ok(toml_str)
    }

    pub fn write(index: &Self) -> Result<()> {
        let str = toml::to_string(&index).unwrap();
        fs::write(Self::path(), str)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Mod {
    pub name: String,
    #[serde(rename = "type")]
    pub project_type: ProjectType,
    pub platform: ModPlatform,
    pub id: String, // curseforge/modrinth id
    pub version: String, // curseforge version ids or modrinth file hashes
    // for a cleaner index, avoid having "pinned = false" listed under every mod
    #[serde(default)] // if value not found use default, bool::default is false
    #[serde(skip_serializing_if = "std::ops::Not::not")] // skip serializing if false, you cant make me create the "is_false" function
    pub pinned: bool
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ModPlatform {
    Modrinth,
    CurseForge,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Mod,
    Datapack,
    Shader,
    Resourcepack,
    // you cant add plugins for now
}

impl TryFrom<MRProjectType> for ProjectType {
    type Error = crate::Error;
    fn try_from(value: MRProjectType) -> std::result::Result<Self, Self::Error> {
        match value {
            MRProjectType::Mod => Ok(Self::Mod),
            MRProjectType::Shader => Ok(Self::Shader),
            MRProjectType::Datapack => Ok(Self::Datapack),
            MRProjectType::ResourcePack => Ok(Self::Resourcepack),
            _ => Err(Error::Parse(format!("{:?} project type is unsupported", value)))
        }
    }
}

// from class_id to readable cf project type
// the cf api documentation sucks..
// https://api.curseforge.com/v1/categories/?gameId=432&classesOnly=true
impl TryFrom<usize> for ProjectType {
    type Error = crate::Error;
    fn try_from(value: usize) -> std::result::Result<Self, Self::Error> {
        match value {            
            6 => Ok(Self::Mod),
            6552 => Ok(Self::Shader),
            6945 => Ok(Self::Datapack),
            12 => Ok(Self::Resourcepack),
            _ => Err(Error::Parse("This curseforge project type is unsupported".to_string()))
        }
    }
}

impl Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", 
            match self {
                ProjectType::Mod => "mod",
                ProjectType::Datapack => "datapack",
                ProjectType::Shader => "shader",
                ProjectType::Resourcepack => "resourcepack",
            })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Modrinthmod {
    pub name: String,
    pub id: String,
    pub version: String
}

impl From<Mod> for Modrinthmod {
    fn from(value: Mod) -> Self {
        Self { name: value.name, id: value.id, version: value.version }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CurseforgeMod {
    pub name: String,
    pub id: i32,
    pub version: i32
}

impl TryFrom<Mod> for CurseforgeMod {
    type Error = crate::Error; 
    fn try_from(value: Mod) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            id: value.id.parse::<i32>().map_err(|_| Error::Parse(format!("curseforge id: {} to Integer", value.id)))?,
            version: value.version.parse::<i32>().map_err(|_| Error::Parse(format!("curseforge version id: {} to Integer", value.version)))?
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ModLoader {
    Fabric,
    Quilt,
    Forge,
    NeoForge
}

impl Display for ModLoader {    
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", 
            match self {
                ModLoader::Fabric => "Fabric",
                ModLoader::Quilt => "Quilt",
                ModLoader::Forge => "Forge",
                ModLoader::NeoForge => "NeoForge", 
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MavenMetadata {
    pub group_id: String,
    pub artifact_id: String,
    pub versioning: MavenVersioning
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MavenVersioning {
    pub release: String,
    pub latest: String,
    pub last_updated: String,
    pub versions: MavenVersion
}

// to be honest I have 0 clue why it works like this
// also calling it with metadata.versioning.versions.version bwuh
#[derive(Serialize, Deserialize, Debug)]
pub struct MavenVersion {
    pub version: Vec<String>
}
