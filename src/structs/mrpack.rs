use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::api::modrinth::SideSupportType;

use super::{index::Side, pack::ModLoader};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub format_version: i32, 
    pub game: Game,
    pub version_id: String,
    pub name: String,
    #[serde(skip_serializing_if="Option::is_none")]
    pub summary: Option<String>,
    pub files: Vec<File>,
    pub dependencies: HashMap<PackDependency, String>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub path: PathBuf,
    pub hashes: FileHashes,
    #[serde(skip_serializing_if="Option::is_none")]
    pub env: Option<FileEnv>,
    pub downloads: Vec<String>,
    pub file_size: usize
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileHashes {
    pub sha1: String,
    pub sha512: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileEnv {
    pub client: SideSupportType,
    pub server: SideSupportType
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum PackDependency {
    Minecraft,
    Forge,
    NeoForge,
    FabricLoader,
    QuiltLoader
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

impl From<Side> for FileEnv {
    fn from(value: Side) -> Self {
        let (client, server) = match value {
            Side::Both => (SideSupportType::Required, SideSupportType::Required),
            Side::Client => (SideSupportType::Required, SideSupportType::Unsupported),
            Side::Server => (SideSupportType::Unsupported, SideSupportType::Required),
        };

        Self {
            client,
            server
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Game {
    #[default]
    Minecraft,
}