use std::{env, fmt::Display, fs, path::PathBuf};
use ferinth::structures::version;
use serde::{Deserialize, Serialize};
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

    pub fn write(modpack: Self) -> Result<()> {
        let str = toml::to_string(&modpack).unwrap();
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Index {
    #[serde(default)]
    pub mods: Vec<Mod>
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
            Self::write(&Index { mods: Vec::new() })?;
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
    pub modrinth_id: Option<String>,
    pub curseforge_id: Option<i32>,
    pub version: String, // curseforge version ids or modrinth file hashes
    pub pinned: Option<bool>
}

impl Mod {
    pub fn new(name: String, modrinth_id: Option<String>, curseforge_id: Option<i32>, version: String, pinned: Option<bool>) -> Self {
        Mod { name, modrinth_id, curseforge_id, version, pinned }
    }

    pub fn seperate_by_platform(self) -> Result<ModByPlatform> {
        if self.modrinth_id.is_some() {
            return Ok(ModByPlatform::ModrinthMod(Modrinthmod {
                name: self.name,
                id: self.modrinth_id.unwrap(),
                version: self.version
            }));
        };

        if self.curseforge_id.is_some() {
            return Ok(ModByPlatform::CurseforgeMod(CurseforgeMod { 
                name: self.name.to_owned(),
                id: self.curseforge_id.unwrap(),
                version: self.version.parse::<i32>().map_err(|_| Error::Parse(format!("Could not parse cf mod version to int {:#?}", self)))?
            }));
        };

        Err(Error::Parse(format!("Something went wrong while trying to parse {:#?}\nas platform specific", self)))
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub enum ModByPlatform {
    ModrinthMod(Modrinthmod),
    CurseforgeMod(CurseforgeMod)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Modrinthmod {
    pub name: String,
    pub id: String,
    pub version: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CurseforgeMod {
    pub name: String,
    pub id: i32,
    pub version: i32
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
