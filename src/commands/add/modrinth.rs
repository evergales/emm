use std::{sync::{Arc, Mutex}, time::Duration};

use async_recursion::async_recursion;
use console::style;
use dialoguer::Select;
use indicatif::ProgressBar;
use tokio::{task::JoinSet, try_join};

use crate::{api::modrinth::{DependencyType, SearchFacet, Version, VersionDependency}, cli::AddModrinthArgs, error::{Error, Result}, structs::{index::{Addon, AddonOptions, AddonSource, Index, ModrinthSource, ProjectType}, pack::Modpack}, util::{modrinth::get_side, FilterVersions}, MODRINTH};

use super::{add_to_index, handle_checked};

pub async fn add_modrinth(args: AddModrinthArgs) -> Result<()> {
    let modpack = Arc::new(Modpack::read()?);
    let progress = ProgressBar::new_spinner().with_message("Adding mods");
    progress.enable_steady_tick(Duration::from_millis(100));

    if args.ids.len() > 1 && args.version.is_some() {
        return Err(Error::Other("Only use the -v/--version flag when adding 1 mod".into()));
    }

    let mut to_search = Vec::new();
    let mut addons = Vec::new();
    for (idx, id) in args.ids.iter().enumerate() {
        if !args.ids.len() == 1 {
            progress.set_message(format!("Adding mods {}/{}", idx + 1, args.ids.len()))
        }
        match resolve_mod(&modpack, id, args.version.as_deref()).await {
            Ok(addon) => addons.push(addon),
            Err(err) => match err {
                Error::NotFound(_) | Error::InvalidId(_) => to_search.push(id.as_str()),
                Error::NoCompatibleVersions(_) | Error::UnsupportedProjectType(_) => println!("{}", style(&err.to_string()).color256(166)), //these are fine to warn and skip
                _ => return Err(err)
            },
        };
    }

    let search_res = search_ids(&modpack, to_search.as_slice(), &progress).await?;
    addons.extend(search_res);

    progress.set_message("Finding dependencies");

    let index = Index::read().await?;
    let checked_ids = Arc::new(Mutex::new(
        // use index & added mods for checked ids as default
        index.addons.iter().map(|m| m.generic_id())
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

    progress.finish_and_clear();
    add_to_index(addons, &index).await?;
    Ok(())
}

async fn resolve_mod(modpack: &Modpack, id: &str, version_id: Option<&str>) -> Result<Addon> {
    let (project, version) = match version_id {
        Some(version_id) => {
            try_join!(
                MODRINTH.get_project(id),
                MODRINTH.get_version(version_id)
            )?
        },
        None => {
            let (project, versions) = try_join!(
                MODRINTH.get_project(id),
                MODRINTH.get_project_versions(id)
            )?;

            match project.project_type {
                ProjectType::Modpack | ProjectType::Plugin => {
                    return Err(Error::UnsupportedProjectType(project.title));
                },
                _ => ()
            }

            let compatible_versions: Vec<Version> = versions.filter_compatible(modpack, &project.project_type);
            if compatible_versions.is_empty() {
                return Err(Error::NoCompatibleVersions(project.title));
            }

            (project, compatible_versions.best_match(modpack).unwrap())
        },
    };

    Ok(Addon {
        name: project.title,
        project_type: project.project_type,
        side: get_side(&project.client_side, &project.server_side),
        source: AddonSource::Modrinth(ModrinthSource {
            id: project.id,
            version: version.id,
        }),
        options: Some(AddonOptions::default()),
        filename: Some(format!("{}.toml", project.slug))
    })
}

async fn search_ids(modpack: &Modpack, strings: &[&str], progress: &ProgressBar) -> Result<Vec<Addon>> {
    let mut results = Vec::new();
    
    for string in strings {
        let facets = vec![
            vec![SearchFacet::Versions(modpack.versions.minecraft.clone())]
        ];

        let hits = MODRINTH.search(string, facets, &20).await?.hits;
        if hits.is_empty() {
            println!("{}", style(format!("Searching for {} returned no results", string)).color256(166));
            continue;
        }

        let titles: Vec<String> = hits.iter().map(|h| h.title.clone()).collect();     

        // if inputed string matches a search result title exactly choose that one
        let chosen = if let Some(exact_match) = titles.iter().position(|t| t.to_lowercase() == string.to_lowercase()) {
            exact_match
        } else {
            let selected = progress.suspend(|| {
                Select::new()
                    .with_prompt(format!("search results for '{}'", string))
                    .items(&titles)
                    .report(false)
                    .interact_opt()
                    .unwrap()
            });

            match selected {
                Some(usize) => usize,
                None => continue, // selecting is cancellable
            }
        };

        let addon = resolve_mod(modpack, &hits[chosen].project_id, None).await?;
        results.push(addon)
    }

    Ok(results)
}

#[async_recursion]
async fn get_dependencies(modpack: &Modpack, addon: &Addon, checked_ids: &Arc<Mutex<Vec<String>>>) -> Result<Vec<Addon>> {
    let mut dependencies = Vec::new();
    let version_id = match &addon.source {
        AddonSource::Modrinth(source) => &source.version,
        _ => unreachable!(),
    };

    let mod_dependencies = MODRINTH.get_version(version_id).await?.dependencies;
    let required_dependencies: Vec<VersionDependency> = mod_dependencies.into_iter().filter(|d| matches!(d.dependency_type, DependencyType::Required)).collect();

    for dep in required_dependencies {
        if dep.project_id.is_none() {
            continue;
        }
        
        // avoid rechecking a dependency multiple times
        if handle_checked(dep.project_id.as_ref().unwrap(), checked_ids) {
            continue;
        }

        // modrinth optionally provides version ids on dependencies so use those
        let resolved_addon = resolve_mod(modpack, &dep.project_id.unwrap(), dep.version_id.as_deref()).await?;
        dependencies.extend(get_dependencies(modpack, &resolved_addon, checked_ids).await?);
        dependencies.push(resolved_addon)
    }

    Ok(dependencies)
}