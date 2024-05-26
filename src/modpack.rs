use std::{env, fs, path::PathBuf};

use crate::{structs::{CurseforgeMod, Index, Mod, ModByPlatform, ModLoader, Modpack, Modrinthmod}, Error, Result};

impl Modpack {
    pub fn new(name: String, author: String, game_version: String, mod_loader: ModLoader) -> Self {
        Modpack { name, author, game_version, mod_loader }
    }

    fn path() -> PathBuf {
        env::current_dir().unwrap().join("mcpack.toml")
    }

    pub fn read() -> Result<Self> {
        if !Self::path().is_file() { return Err(Error::Uninitialized); }
        let packfile_str = &fs::read_to_string(Self::path()).map_err(|_| Error::Uninitialized)?;
        let toml_str = toml::from_str(packfile_str).map_err(|err| Error::Parse(format!("Error while parsing config to toml {err}")))?;
        Ok(toml_str)
    }

    pub fn write(modpack: Self) -> Result<()> {
        let str = toml::to_string(&modpack).unwrap();
        fs::write(Self::path(), str)?;
        Ok(())
    }
}

impl Index {
    fn path() -> PathBuf {
        env::current_dir().unwrap().join("index.toml")
    }

    pub fn read() -> Result<Self> {
        if !Modpack::path().is_file() {
            return Err(Error::Uninitialized);
        }
        if !Self::path().is_file() {
            Self::write(&Index { mods: Vec::new() })?;
        }
        let index_str = &fs::read_to_string(Self::path()).map_err(|_| Error::Uninitialized)?;
        let toml_str = toml::from_str(index_str).map_err(|err| Error::Parse(format!("Error while parsing index to toml {err}")))?;
        Ok(toml_str)
    }

    pub fn write(index: &Self) -> Result<()> {
        let str = toml::to_string(&index).unwrap();
        fs::write(Self::path(), str)?;
        Ok(())
    }

    pub fn add_mods(mods: Vec<Mod>) -> Result<()> {
        let mut index = Self::read()?;
        for m in mods {
            // checking the name as well so you cant add the same mod from both modrinth or curseforge
            if index.mods.iter().any(|idx_mod| idx_mod.name == m.name || *idx_mod == m) {
                println!("{} is already in the modpack!", m.name);
                continue;
            }
            println!("Adding {}!", m.name);
            index.mods.push(m)
        }

        index.mods.sort_by_key(|m| m.name.to_owned());
        Self::write(&index)?;
        Ok(())
    }
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

impl Mod {
    pub fn new(name: String, modrinth_id: Option<String>, curseforge_id: Option<i32>, version: String, pinned: Option<bool>) -> Self {
        Mod { name, modrinth_id, curseforge_id, version, pinned }
    }

    pub fn seperate_by_platform(self) -> Result<ModByPlatform> {
        if self.modrinth_id.is_some() {
            return Ok(ModByPlatform::ModrinthMod(Modrinthmod {
                name: self.name,
                id: self.modrinth_id.unwrap(),
                version: self.version
            }));
        };

        if self.curseforge_id.is_some() {
            return Ok(ModByPlatform::CurseforgeMod(CurseforgeMod { 
                name: self.name.to_owned(),
                id: self.curseforge_id.unwrap(),
                version: self.version.parse::<i32>().map_err(|_| Error::Parse(format!("Could not parse cf mod version to int {:#?}", self)))?
            }));
        };

        Err(Error::Parse(format!("Something went wrong while trying to parse {:#?}\nas platform specific", self)))
    }
}