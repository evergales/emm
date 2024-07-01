pub mod versions;
pub mod files;

use ferinth::structures::version::VersionFile;
use tokio::task::JoinSet;

use crate::{structs::{CurseforgeMod, Mod, ModPlatform, Modrinthmod}, Result};

pub async fn join_all(mut set: JoinSet<Result<()>>) -> Result<()> {
    while let Some(res) = set.join_next().await {
        let _ = res?;
    }
    Ok(())
}

pub fn primary_file(files: Vec<VersionFile>) -> VersionFile {
    files.into_iter().find(|f| f.primary).unwrap()
}

// determine if a mod matches a name or id 
pub fn mod_matches(m: &Mod, s: &String) -> bool {
    // names set to lowercase to make matching less case sensitive
    if m.name.to_lowercase() == s.to_lowercase() { return true; }

    match m.platform {
        ModPlatform::Modrinth => {return &m.id == s},
        ModPlatform::CurseForge => {
            if let Ok(id) = s.parse::<i32>() {
                return m.id.parse::<i32>().unwrap_or_default() == id;
            }
        },
    }
    
    false // I guess if it doesnt have a modrinth or curseforge id this is here
}

pub async fn seperate_mods_by_platform(mods: Vec<Mod>) -> Result<(Vec<Modrinthmod>, Vec<CurseforgeMod>)> {
    let mut mr_mods: Vec<Modrinthmod> = Vec::new();
    let mut cf_mods: Vec<CurseforgeMod> = Vec::new();

    for i in mods {
        match i.platform {
            ModPlatform::Modrinth => {
                mr_mods.push(Modrinthmod::from(i));
            },
            ModPlatform::CurseForge => {
                cf_mods.push(CurseforgeMod::try_from(i)?)
            },
        }
    }

    Ok((mr_mods, cf_mods))
}