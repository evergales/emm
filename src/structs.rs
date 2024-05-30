use std::fmt::Display;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Modpack {
    pub about: ModpackAbout,
    pub versions: ModpackVersions,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModpackAbout {
    pub name: String,
    pub authors: Vec<String>,
    pub description: Option<String>
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
    pub(crate) mods: Vec<Mod>
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Mod {
    pub name: String,
    pub modrinth_id: Option<String>,
    pub curseforge_id: Option<i32>,
    pub version: String, // curseforge version ids or modrinth file hashes
    pub pinned: Option<bool>
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