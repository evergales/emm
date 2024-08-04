use serde::{Deserialize, Serialize};

use crate::error::Result;

const NEOFORGE_MAVEN_URL: &str = "https://maven.neoforged.net/api/maven";

pub async fn get_supported_versions(mc_version: &str) -> Result<Vec<String>> {
    let filtered_versions_url = match mc_version {
        "1.20.1" => format!("{}/versions/releases/net/neoforged/forge?filter=1.20.1", NEOFORGE_MAVEN_URL),
        _ => format!("{}/versions/releases/net/neoforged/neoforge?filter={}", NEOFORGE_MAVEN_URL, mc_version.strip_prefix("1.").unwrap_or("unsupported"))
    };

    let res: Versions = reqwest::get(filtered_versions_url).await?.json().await?;
    Ok(res.versions)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Versions {
    is_snapshot: bool,
    versions: Vec<String>
}