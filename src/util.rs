use tokio::task::JoinSet;

use crate::{structs::{CurseforgeMod, MavenMetadata, Mod, ModByPlatform, ModLoader, Modrinthmod}, Error, Result};

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

pub async fn join_all(mut set: JoinSet<Result<()>>) -> Result<()> {
    while let Some(res) = set.join_next().await {
        let _ = res?;
    }
    Ok(())
}

pub async fn get_compatible_loader_versions(loader: &ModLoader, mc_version: &String) -> Result<Vec<String>> {
    let metadata = loader.get_version_maven().await?;

    // todo: make beta versions of Quilt and NeoForge accessible if the user wants them
    let compatible_versions = match loader {
        // just get latest on fabric
        ModLoader::Fabric | ModLoader::Quilt => metadata.versioning.versions.version,
        ModLoader::Forge => {
            // versions are formatted "{mc_version}-{forge_version}"
            metadata.versioning.versions.version.into_iter().filter(|v| v.starts_with(mc_version)).collect()
        },
        ModLoader::NeoForge => {
            // this doesnt allow for 1.20.1 versions because they're on a whole different maven
            // and they're "legacy" and aaaaaaaaaaaaaa

            // versions are formatted "{mc_version without major (withour '1.')}.{neoforge_version}"
            let version = mc_version.strip_prefix("1.").unwrap(); // not really sure about this but it works
            metadata.versioning.versions.version.into_iter().filter(|v| v.starts_with(version)).collect()
        },
    };

    Ok(compatible_versions)
}

pub async fn get_latest_loader_version(loader: &ModLoader, mc_version: &String) -> Result<String> {
    let versions = get_compatible_loader_versions(loader, mc_version).await?;
    if versions.is_empty() {
        return Err(Error::Other(format!("{} doesn't have compatible versions with minecraft {}", loader, mc_version)));
    }

    let latest_version = match loader {
        // just get latest on fabric
        ModLoader::Fabric => versions.last().unwrap().to_owned(),
        ModLoader::Quilt => { 
            let releases: Vec<String> = versions.into_iter().filter(|v| !v.contains("beta")).collect();
            releases.last().unwrap().to_owned()
        },
        ModLoader::Forge => {
            versions[0].strip_prefix(&format!("{mc_version}-")).unwrap().to_owned()
        },
        ModLoader::NeoForge => {
            versions[0].to_owned()
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