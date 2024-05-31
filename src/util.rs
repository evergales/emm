use serde::{Deserialize, Serialize};

use crate::{structs::{CurseforgeMod, Mod, ModByPlatform, ModLoader, Modrinthmod}, Error, Result};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MavenMetadata {
    pub group_id: String,
    pub artifact_id: String,
    pub versioning: MavenVersioning
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MavenVersioning {
    pub release: String,
    pub latest: String,
    pub last_updated: String,
    pub versions: MavenVersion
}

// to be honest I have 0 clue why it works like this
// also calling it with metadata.versioning.versions.version bwuh
#[derive(Serialize, Deserialize, Debug)]
pub struct MavenVersion {
    pub version: Vec<String>
}

impl ModLoader {
    pub async fn get_version_maven(&self) -> Result<MavenMetadata> {
        let url = match self {
            ModLoader::Fabric => "https://maven.fabricmc.net/net/fabricmc/fabric-loader/maven-metadata.xml",
            ModLoader::Quilt => "https://maven.quiltmc.org/repository/release/org/quiltmc/quilt-loader/maven-metadata.xml",
            ModLoader::Forge => "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml",
            ModLoader::NeoForge => "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml",
        }.to_string();

        let str = reqwest::get(url).await?.text().await?;
        let metadata: MavenMetadata = serde_xml_rs::from_str(&str).expect("could not parse Maven Metadata");
        Ok(metadata)
    }
}

pub async fn get_latest_loader_version(loader: &ModLoader, mc_version: &String) -> Result<String> {
    let metadata = loader.get_version_maven().await?;

    let latest_version = match loader {
        ModLoader::Fabric => metadata.versioning.latest,
        ModLoader::Quilt => { 
            let releases = metadata.versioning.versions.version.into_iter().filter(|v| !v.contains("beta")).collect::<Vec<String>>();
            releases.last().unwrap().to_owned()
        },
        ModLoader::Forge => {
            let compatible: Vec<String> = metadata.versioning.versions.version.into_iter().filter(|v| v.starts_with(mc_version)).collect();
            if compatible.is_empty() { return Err(Error::Other("there are no MinecraftForge loader versions for your minecraft version available".to_string())) }
            compatible[0].strip_prefix(&format!("{mc_version}-")).unwrap().to_owned()
        },
        ModLoader::NeoForge => {
            // this doesnt allow for 1.20.1 versions because they're on a whole different maven
            // and they're "legacy" and aaaaaaaaaaaaaa
            let version = mc_version.strip_prefix("1.").unwrap(); // not really sure about this but it works
            let compatible: Vec<String> = metadata.versioning.versions.version.into_iter().filter(|v| v.starts_with(version)).collect();
            if compatible.is_empty() { return Err(Error::Other("there are no NeoForge loader versions for your minecraft version available".to_string())) }
            compatible[0].to_owned()
        },
    };

    Ok(latest_version)
}

pub async fn seperate_mods_by_platform(mods: Vec<Mod>) -> Result<(Vec<Modrinthmod>, Vec<CurseforgeMod>)> {
    let mut mr_mods: Vec<Modrinthmod> = Vec::new();
    let mut cf_mods: Vec<CurseforgeMod> = Vec::new();

    for i in mods {
        match i.seperate_by_platform()? {
            ModByPlatform::ModrinthMod(mr_mod) => mr_mods.push(mr_mod),
            ModByPlatform::CurseforgeMod(cf_mod) => cf_mods.push(cf_mod),
        }
    }

    Ok((mr_mods, cf_mods))
}