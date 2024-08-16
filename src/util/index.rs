use std::{env, fs, path::PathBuf, sync::Arc};

use dialoguer::{FuzzySelect, Select};
use tokio::{sync::Semaphore, task::JoinSet};

use crate::{error::{Error, Result}, structs::{index::{Addon, AddonSource, Index}, pack::Modpack}};

use super::files::is_local_path;

impl Addon {
    fn index_file_name(&self) -> String {
        format!("{}.toml", self.name.to_lowercase().replace(' ', "-"))
    }

    pub fn generic_id(&self) -> String {
        match &self.source {
            AddonSource::Modrinth(source) => source.id.clone(),
            AddonSource::Curseforge(source) => source.id.to_string(),
            AddonSource::Github(source) => source.repo.clone(),
        }
    }

    // whether a string matches the addon's name/id
    pub fn matches_str(&self, string: &str) -> bool {
        string.to_lowercase() == self.name.to_lowercase() || string == self.generic_id()
    }
}

impl Index {
    pub fn path() -> Result<PathBuf> {
        let index_path = Modpack::read()?.index_path;
        if !is_local_path(&index_path) {            
            return Err(Error::Other("Invalid index path, the path to your index folder must be relative and can not leave the project root, for example: './index'".into()));
        }
        Ok(env::current_dir()?.join(index_path))
    }

    pub async fn read() -> Result<Self> {
        let path = Self::path()?;
        if !path.is_dir() {
            return Ok(Index { addons: vec![] });
        }

        let mut tasks: JoinSet<Result<Option<Addon>>> = JoinSet::new();
        let entries = fs::read_dir(path)?;

        for entry in entries {
            let task = async move {
                let entry = entry?;
                let path = entry.path();
                if !path.is_file() || path.extension().unwrap_or_default() != "toml" {
                    return Ok(None);
                }
    
                let content = fs::read_to_string(&path)?;
                let mut addon: Addon = toml::from_str(&content).unwrap();
                addon.filename = Some(path.file_name().unwrap().to_string_lossy().to_string());

                Ok(Some(addon))
            };

            tasks.spawn(task);
        }

        let mut addons = Vec::new();

        while let Some(res) = tasks.join_next().await {
            if let Some(addon) = res?? {
                addons.push(addon)
            }
        }

        Ok(Index { addons })
    }

    pub async fn write_addons(addons: Vec<Addon>) -> Result<()> {
        let path = Arc::new(Self::path()?);
        if !path.is_dir() {
            fs::create_dir_all(&*path)?;
        }

        let mut tasks: JoinSet<Result<()>> = JoinSet::new();
        let permits = Arc::new(Semaphore::new(50));

        for addon in addons {
            let path = path.clone();
            let permits = permits.clone();

            let task = async move {
                let _permit = permits.acquire().await.unwrap();
                let file_name = addon.index_file_name();
                fs::write(path.join(file_name), toml::to_string_pretty(&addon).unwrap())?;
                Ok(())
            };

            tasks.spawn(task);
        }

        while let Some(res) = tasks.join_next().await { res?? }

        Ok(())
    }

    pub async fn remove_addons(addons: &[&Addon]) -> Result<()> {
        for addon in addons {
            let filename = addon.filename.clone().unwrap_or(addon.index_file_name());
            let path = Self::path()?.join(filename);
            if !path.is_file() {
                return Err(Error::Other(format!("Could not remove {} from index as its file was not found", addon.name)));
            }

            fs::remove_file(path)?;
        }

        Ok(())
    }

    pub fn select_addon(&self, str: &str) -> Option<&Addon> {
        match self.addons.iter().find(|a| a.matches_str(str)) {
            Some(addon) => Some(addon),
            None => {
                let idx = FuzzySelect::new()
                    .with_prompt("Similar to:")
                    .items(&self.addons.iter().map(|a| a.name.as_str()).collect::<Vec<&str>>())
                    .with_initial_text(str)
                    .report(false)
                    .interact_opt()
                    .unwrap();

                idx.map(|idx| self.addons.get(idx).unwrap())
            },
        }
    }
}