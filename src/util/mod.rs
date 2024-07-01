pub mod versions;

use std::{fs::File, io::{Read, Write}, path::PathBuf};

use ferinth::structures::version::VersionFile;
use tokio::task::JoinSet;
use walkdir::WalkDir;
use zip::{write::SimpleFileOptions, ZipWriter};

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

// https://github.com/zip-rs/zip2/blob/master/examples/write_dir.rs
pub fn add_recursively(from_path: PathBuf, zip_path: PathBuf, zip: &mut ZipWriter<File>, options: SimpleFileOptions) -> zip::result::ZipResult<()> {
    let mut buffer = Vec::new();
    for entry in WalkDir::new(&from_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let file_name = path.strip_prefix(&from_path).unwrap();
        let path_as_string = file_name.to_str().to_owned().unwrap();

        if path.is_file() {
            zip.start_file_from_path(zip_path.join(path_as_string), options)?;
            let mut f = File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
            buffer.clear()
        }
    }

    Ok(())
}

pub async fn download_file(path: &PathBuf, url: &String) -> Result<()> {
    let res = reqwest::get(url).await?;
    let data = &*res.bytes().await?;
    let mut file = File::create(path)?;
    file.write_all(data)?;
    Ok(())
}