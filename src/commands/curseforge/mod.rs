use serde::{Deserialize, Serialize};

pub mod export;

// I honestly could not find documentation on curseforge packs
// this is me looking at the manifest file and trying to match it..

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CfManifest {
    minecraft: CfMinecraft,
    manifest_type: String,
    manifest_version: i32,
    name: String,
    version: String,
    author: String,
    files: Vec<CfFile>,
    overrides: String
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CfMinecraft {
    version: String,
    mod_loaders: Vec<CfModLoader>
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CfModLoader {
    id: String,
    primary: bool
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CfFile {
    #[serde(rename = "projectID")]
    project_id: i32,
    #[serde(rename = "fileID")]
    file_id: i32,
    required: bool
}