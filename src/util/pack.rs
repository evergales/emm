use std::{env, fs, path::PathBuf};

use crate::{error::{Error, Result}, structs::pack::Modpack};

use super::versions::get_latest_loader_version;

impl Modpack {
    pub fn path() -> PathBuf {
        env::current_dir().unwrap().join("pack.toml")
    }

    pub fn read() -> Result<Self> {
        if !Self::path().is_file() { return Err(Error::Uninitialized); }
        let packfile_str = &fs::read_to_string(Self::path()).map_err(|_| Error::Uninitialized)?;
        let toml_str = toml::from_str(packfile_str).unwrap();
        Ok(toml_str)
    }

    pub fn write(modpack: &Self) -> Result<()> {
        let str = toml::to_string(modpack).unwrap();
        fs::write(Self::path(), str)?;
        Ok(())
    }

    pub async fn get_loader_version(&self) -> Result<String> {
        match self.versions.loader_version.as_str() {
            "latest" => get_latest_loader_version(&self.versions.loader, &self.versions.minecraft).await,
            _ => Ok(self.versions.loader_version.clone())
        }
    }
}