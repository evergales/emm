use std::collections::HashMap;

use crate::Result;

const FORGE_METADATA_URL: &str = "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json";

async fn fetch_versions() -> Result<HashMap<String, Vec<String>>> {
    let versions: HashMap<String, Vec<String>> = reqwest::get(FORGE_METADATA_URL).await?.json().await?;
    Ok(versions)
}

// get supported loader versions for mc version
pub async fn get_supported_versions(mc_version: &String) -> Result<Vec<String>> {
    let versions = fetch_versions().await?;
    if let Some(supported_versions) = versions.get_key_value(mc_version) {
        let formatted: Vec<String> = supported_versions.1.iter()
            .map(|v| {
                v.strip_prefix(&(mc_version.to_owned() + "-"))
                    .unwrap()
                    .to_owned()
            })
            .collect();
        Ok(formatted)
    } else {
        Ok(Vec::new())
    }
}
