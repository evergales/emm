use serde::{Deserialize, Serialize};

use crate::Result;

const FABRIC_META_URL: &str = "https://meta.fabricmc.net/";

pub async fn fetch_supported_mc_versions() -> Result<Vec<String>> {
    let versions: Vec<GameVersion> = reqwest::get(FABRIC_META_URL.to_owned() + "/v2/versions/game").await?.json().await?;
    Ok(versions.into_iter().map(|v| v.version).collect())
}

pub async fn fetch_loader_versions() -> Result<Vec<String>> {
    let loaders: Vec<LoaderVersion> = reqwest::get(FABRIC_META_URL.to_owned() + "/v2/versions/loader").await?.json().await?;
    Ok(loaders.into_iter().map(|l| l.version).collect())
}

#[derive(Debug, Serialize, Deserialize)]
struct GameVersion {
    version: String,
    stable: bool
}

#[derive(Debug, Serialize, Deserialize)]
struct LoaderVersion {
    separator: String,
    build: u32,
    maven: String,
    version: String,
    stable: bool
}