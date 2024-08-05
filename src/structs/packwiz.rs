use std::{collections::HashMap, default, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::index::Side;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PwPack {
    pub name: String,
    pub author: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub pack_format: String,
    pub index: PwIndexInfo,
    pub versions: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PwIndexInfo {
    pub file: String,
    pub hash_format: HashFormat,
    pub hash: String
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum HashFormat {
    Sha512,
    Sha256,
    Sha1,
    Md5,
    Murmur2
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PwIndex {
    pub hash_format: HashFormat,
    pub files: Vec<IndexFile>
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct IndexFile {
    pub file: String,
    pub hash: String,
    pub hash_format: Option<HashFormat>,
    pub metafile: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PwModHelper {
    pub file_path: PathBuf,
    pub hash: String,
    pub pwmod_str: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PwMod {
    pub name: String,
    pub filename: String,
    pub download: ModDownload,
    pub option: Option<ModOptions>,
    pub side: Option<Side>,
    pub update: Option<ModUpdate>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ModDownload {
    pub url: Option<String>,
    pub hash_format: HashFormat,
    pub hash: String,
    pub mode: Option<DownloadMode>
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadMode {
    Url,
    #[serde(rename = "metadata:curseforge")]
    Curseforge
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModUpdate {
    pub modrinth: Option<ModrinthModUpdate>,
    pub curseforge: Option<CurseforgeModUpdate>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ModrinthModUpdate {
    pub mod_id: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CurseforgeModUpdate {
    pub project_id: i32,
    pub file_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModOptions {
    pub optional: bool,
    pub default: Option<bool>,
    pub description: Option<String>,
}