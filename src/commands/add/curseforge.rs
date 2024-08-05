use std::sync::{Arc, Mutex};

use async_recursion::async_recursion;
use console::style;
use dialoguer::Select;
use tokio::{task::JoinSet, try_join};

use crate::{api::curseforge::{File, FileDependency, FileRelationType}, error::{Error, Result}, structs::{index::{Addon, AddonOptions, AddonSource, CurseforgeSource, Index, ProjectType, Side}, pack::Modpack}, CURSEFORGE};

use super::{add_to_index, handle_checked};

pub async fn add_curseforge(ids: Vec<String>, version: Option<i32>) -> Result<()> {
    let modpack = Arc::new(Modpack::read()?);

    if ids.len() > 1 && version.is_some() {
        return Err(Error::Other("Only use the -v/--version flag when adding 1 mod".into()));
    }

    let mut to_search = Vec::new();
    let mut addons = Vec::new();
    for (idx, id) in ids.iter().enumerate() {
        match resolve_mod(&modpack, id, version).await {
            Ok(addon) => addons.push(addon),
            Err(err) => match err {
                Error::NotFound(_) | Error::InvalidId(_) => to_search.push(id.as_str()),
                _ => return Err(err)
            },
        };
    }

    addons.extend(search_ids(&modpack, to_search.as_slice()).await?);

    let index_addons = Index::read().await?.addons;
    let checked_ids = Arc::new(Mutex::new(
        // use index & added mods for checked ids as default
        index_addons.iter().map(|m| m.generic_id())
            .chain(addons.iter().map(|m| m.generic_id()))
            .collect()
    ));
    let mut tasks: JoinSet<Result<Vec<Addon>>> = JoinSet::new();

    for addon in addons.clone() {
        let modpack = modpack.clone();
        let checked_ids = checked_ids.clone();

        let task = async move {
            get_dependencies(&modpack, &addon, &checked_ids).await
        };

        tasks.spawn(task);
    }

    // wait for tasks to finish and push dependencies to addons
    while let Some(res) = tasks.join_next().await { addons.extend(res??) }

    add_to_index(addons).await?;
    Ok(())
}

async fn resolve_mod(modpack: &Modpack, id: &str, version_id: Option<i32>) -> Result<Addon> {
    let (cf_mod, files) = if let Ok(id) = id.parse::<i32>() {
        match version_id {
            Some(version_id) => {
                let (cf_mod, file) = try_join!(
                    CURSEFORGE.get_mod(&id),
                    CURSEFORGE.get_mod_file(&id, &version_id)
                )?;

                (cf_mod, vec![file])
            },
            None => {
                try_join!(
                    CURSEFORGE.get_mod(&id),
                    CURSEFORGE.get_mod_files(&id)
                )?
            },
        }
    } else {
        let cf_mod = CURSEFORGE.get_mod_by_slug(id).await?;
        let files = CURSEFORGE.get_mod_files(&cf_mod.id).await?;

        (cf_mod, files)
    };

    let project_type = ProjectType::try_from(cf_mod.class_id.unwrap()).map_err(|_| Error::UnsupportedProjectType(cf_mod.name.clone()))?;

    let compatibles = files.into_iter().filter(|f| 
        f.is_available
        && if matches!(project_type, ProjectType::Mod) { f.game_versions.contains(&modpack.versions.loader.to_string()) } else { true }
        && f.game_versions.contains(&modpack.versions.minecraft)
    ).collect::<Vec<File>>();

    if compatibles.is_empty() {
        return Err(Error::Other(style(format!("No compatible versions found for mod: '{}'", cf_mod.name)).color256(166).to_string()));
    }

    Ok(Addon {
        name: cf_mod.name,
        project_type,
        side: Side::Both,
        source: AddonSource::Curseforge(CurseforgeSource {
            id: cf_mod.id,
            version: compatibles.first().unwrap().id,
        }),
        options: Some(AddonOptions::default()),
        filename: None
    })
}

async fn search_ids(modpack: &Modpack, strings: &[&str]) -> Result<Vec<Addon>> {
    let mut results = Vec::new();
    
    for string in strings {
        let hits = CURSEFORGE.search(string, &modpack.versions.minecraft, &modpack.versions.loader, &20).await?;
        if hits.is_empty() {
            println!("{}", style(format!("Searching for {} returned no results", string)).color256(166));
            continue;
        }

        let titles: Vec<String> = hits.iter().map(|m| m.name.clone()).collect();     

        // if inputed string matches a search result title exactly choose that one
        let chosen = if let Some(exact_match) = titles.iter().position(|t| t.to_lowercase() == string.to_lowercase()) {
            exact_match
        } else {
            let selected = Select::new()
                    .with_prompt(format!("search results for '{}'", string))
                    .items(&titles)
                    .interact_opt()
                    .unwrap();

            match selected {
                Some(usize) => usize,
                None => continue, // selecting is cancellable
            }
        };

        let addon = resolve_mod(modpack, &hits[chosen].id.to_string(), None).await?;
        results.push(addon)
    }

    Ok(results)
}

#[async_recursion]
async fn get_dependencies(modpack: &Modpack, addon: &Addon, checked_ids: &Arc<Mutex<Vec<String>>>) -> Result<Vec<Addon>> {
    let mut dependencies = Vec::new();
    let source = match &addon.source {
        AddonSource::Curseforge(source) => source,
        _ => unreachable!()
    };

    let file = CURSEFORGE.get_mod_file(
    &source.id,
    &source.version
    ).await?;

    let mod_dependencies: Vec<FileDependency> = file.dependencies.into_iter().filter(|f| matches!(f.relation_type, FileRelationType::RequiredDependency)).collect();
    
    for dep in mod_dependencies {
        // avoid rechecking a dependency multiple times
        if handle_checked(&dep.mod_id.to_string(), checked_ids) {
            continue;
        }
        
        let resolved_addon = resolve_mod(modpack, &dep.mod_id.to_string(), None).await?;
        dependencies.extend(get_dependencies(modpack, &resolved_addon, checked_ids).await?);
        dependencies.push(resolved_addon)
    }

    Ok(dependencies)
}