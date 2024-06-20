pub mod import;
pub mod export;

use std::{collections::HashMap, path::PathBuf};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    /// The version of the format, stored as a number.
    /// The current value at the time of writing is 1.
    pub format_version: i32,
    /// The game of the modpack, stored as a string.
    /// Currently the only available type is minecraft. 
    pub game: Game,
    /// A unique identifier for this specific version of the modpack.
    pub version_id: String,
    /// Human-readable name of the modpack.
    pub name: String,
    /// A short description of this modpack.
    #[serde(skip_serializing_if="Option::is_none")]
    pub summary: Option<String>,
    /// Contains a list of files for the modpack that needs to be downloaded.
    pub files: Vec<File>,
    /// This object contains a list of IDs and version numbers that launchers will use in order to know what to install.
    pub dependencies: HashMap<PackDependency, String>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct File {
    /// The destination path of this file, relative to the Minecraft instance directory.
    /// For example, mods/MyMod.jar resolves to .minecraft/mods/MyMod.jar.
    pub path: PathBuf,
    /// The hashes of the file specified.
    /// This MUST contain the SHA1 hash and the SHA512 hash.
    pub hashes: FileHashes,
    /// For files that only exist on a specific environment, this field allows that to be specified.
    /// This uses the Modrinth client/server type specifications.
    #[serde(skip_serializing_if="Option::is_none")]
    pub env: Option<FileEnv>,
    /// An array containing HTTPS URLs where this file may be downloaded.
    /// When uploading to Modrinth, the pack is validated so that only URIs from the following domains are allowed:
    /// ```
    /// cdn.modrinth.com
    /// github.com
    /// raw.githubusercontent.com
    /// gitlab.com
    /// ```
    pub downloads: Vec<Url>,
    /// An integer containing the size of the file, in bytes.
    pub file_size: usize
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileHashes {
    pub sha1: String,
    pub sha512: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileEnv {
    pub client: EnvSideType,
    pub server: EnvSideType
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum EnvSideType {
    Required,
    Optional,
    Unsupported,
    Unknown
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

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Game {
    #[default]
    Minecraft,
}