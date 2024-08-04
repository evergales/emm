use std::path::PathBuf;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Modpack {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub index_path: PathBuf,
    pub options: PackOptions,
    pub versions: Versions
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct PackOptions {
    pub acceptable_versions: Option<Vec<String>>,
    pub overrides_path: Option<PathBuf>,
    pub mods_output: Option<PathBuf>,
    pub resourcepacks_output: Option<PathBuf>,
    pub shaders_output: Option<PathBuf>,
    pub datapacks_output: Option<PathBuf>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Versions {
    pub minecraft: String,
    pub loader: ModLoader,
    pub loader_version: String
}

#[derive(Debug, Serialize, Deserialize, Clone, ValueEnum, PartialEq)]
#[serde(rename_all = "lowercase")]
#[clap(rename_all = "lowercase")]
pub enum ModLoader {
    Fabric,
    Quilt,
    Forge,
    NeoForge
}

impl std::fmt::Display for ModLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Fabric => "Fabric",
            Self::Quilt => "Quilt",
            Self::Forge => "Forge",
            Self::NeoForge => "Neoforge",
        })
    }
}