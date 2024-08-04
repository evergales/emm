use serde::{Deserialize, Serialize};

use crate::error::Result;

const PISTOR_META_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

async fn fetch_version_manifest() -> Result<VersionManifest> {
    let version_manifest: VersionManifest = reqwest::get(PISTOR_META_URL).await?.json().await?;
    Ok(version_manifest)
}

pub async fn get_latest_release() -> Result<String> {
    Ok(fetch_version_manifest().await?.latest.release)
}

pub async fn get_latest_snapshot() -> Result<String> {
    Ok(fetch_version_manifest().await?.latest.snapshot)
}

pub async fn list_mc_versions(filter: Option<VersionType>) -> Result<Vec<String>> {
    let version_manifest = fetch_version_manifest().await?;
    if filter.is_some() {
        let filtered_versions: Vec<String> = version_manifest.versions
            .into_iter()
            .filter_map(|v| {
                if &v.version_type == filter.as_ref().unwrap() {
                    Some(v.id)
                } else {
                    None
                }
            })
            .collect();

        Ok(filtered_versions)
    } else {
        Ok(version_manifest.versions.into_iter().map(|v| v.id).collect())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionManifest {
    latest: LatestVersions,
    versions: Vec<ManifestVersion>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LatestVersions {
    release: String,
    snapshot: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestVersion {
    id: String,
    #[serde(rename = "type")]
    version_type: VersionType,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VersionType {
    Release,
    Snapshot,
    OldAlpha,
    OldBeta,
}
