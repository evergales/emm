use std::fmt::Display;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Modpack {
    pub name: String,
    pub author: String,
    pub game_version: String,
    pub mod_loader: ModLoader,
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