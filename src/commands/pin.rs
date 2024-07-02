use crate::{structs::{CurseforgeMod, Index, ModPlatform, Modpack}, util::mod_matches, Result, CURSEFORGE, MODRINTH};

pub async fn pin(mod_str: String, version_id: Option<String>) -> Result<()> {
    let modpack = Modpack::read()?;
    let mut index = Index::read()?;

    let index_mod = match index.mods.iter().find(|m| mod_matches(m, &mod_str)) {
        Some(m) => m,
        None => {
            println!("Could not find {mod_str} in this modpack!");
            return Ok(());
        }
    };

    let version: Option<String> = match &version_id {
        Some(version_id) => match index_mod.clone().platform {
            ModPlatform::Modrinth => {
                let version = MODRINTH.get_version(version_id).await?;
                let compatible = {
                    version.loaders.contains(&modpack.versions.mod_loader.to_string().to_lowercase())
                    && version.game_versions.contains(&modpack.versions.minecraft)
                };


                if !compatible {
                    println!("The version id you provided is incompatible with your modpack");
                    return Ok(());
                }

                Some(version.id)
            }
            ModPlatform::CurseForge => {
                let cf_mod = CurseforgeMod::try_from(index_mod.to_owned())?;
                let version_id = match version_id.parse::<i32>() {
                    Ok(id) => id,
                    Err(_) => {
                        println!("The version id you provided is invalid");
                        return Ok(());
                    },
                };

                let file = CURSEFORGE.get_mod_file(cf_mod.id, version_id).await?;
                let compatible: bool = {
                    file.is_available
                    && file.game_versions.contains(&modpack.versions.mod_loader.to_string())
                    && file.game_versions.contains(&modpack.versions.minecraft)
                };

                if !compatible {
                    println!("The version id you provided is incompatible with your modpack");
                    return Ok(());
                }

                Some(file.id.to_string())
            },
        },
        None => None,
    };

    let idx = index.mods.iter().position(|m| m == index_mod).unwrap();
    index.mods[idx].pinned = true;
    if version.is_some() {
        index.mods[idx].version = version.clone().unwrap()
    }

    Index::write(&index)?;
    println!("Pinning {} {}", index.mods[idx].name, {
        if version.is_some() { format!("to {}", version.unwrap()) } else {"".to_string()}
    });
    Ok(())
}