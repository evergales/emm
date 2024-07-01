use crate::{structs::ModLoader, Error, Result};

pub mod fabric;
pub mod forge;
pub mod minecraft;
pub mod neoforge;
pub mod quilt;

pub async fn get_compatible_loader_versions(loader: &ModLoader, mc_version: &String) -> Result<Vec<String>> {
    match loader {
        ModLoader::Fabric => {
            if !fabric::fetch_supported_mc_versions().await?.contains(mc_version) {
                return Err(Error::NoLoaderSupport(loader.to_string(), mc_version.to_owned()));
            }

            Ok(fabric::fetch_loader_versions().await?)
        },
        ModLoader::Quilt => {
            if !quilt::fetch_supported_mc_versions().await?.contains(mc_version) {
                return Err(Error::NoLoaderSupport(loader.to_string(), mc_version.to_owned()));
            }

            Ok(quilt::fetch_loader_versions().await?)
        },
        ModLoader::Forge => {
            let versions = forge::get_supported_versions(mc_version).await?;
            if versions.is_empty() {
                return Err(Error::NoLoaderSupport(loader.to_string(), mc_version.to_owned()));
            }
            
            Ok(versions)
        },
        ModLoader::NeoForge => {
            let versions = neoforge::get_supported_versions(mc_version).await?;
            if versions.is_empty() {
                return Err(Error::NoLoaderSupport(loader.to_string(), mc_version.to_owned()));
            }

            Ok(versions)
        },
    }
}

pub async fn get_latest_loader_version(loader: &ModLoader, mc_version: &String) -> Result<String> {
    let versions = get_compatible_loader_versions(loader, mc_version).await?;

    Ok(match loader {
        ModLoader::Fabric | ModLoader::Quilt => versions.first().unwrap(),
        ModLoader::Forge | ModLoader::NeoForge => versions.last().unwrap(),
    }.to_owned())
}