use std::{sync::Arc, time::Duration};

use async_recursion::async_recursion;
use dialoguer::Select;
use ferinth::structures::{project::ProjectType as MRProjectType, search::{Facet, Sort}, version::{Dependency, DependencyType, Version}};
use furse::structures::file_structs::{File, FileDependency, FileRelationType};
use indicatif::ProgressBar;
use tokio::{sync::Mutex, task::JoinSet, try_join};

use crate::{structs::{Index, Mod, ModPlatform, Modpack, ProjectType}, util::join_all, Error, Result, CURSEFORGE, MODRINTH};

pub async fn add_mods(ids: Vec<String>, version: Option<String>) -> Result<()> {
    let modpack = Modpack::read()?;
    let progress = ProgressBar::new_spinner().with_message("Adding mods");
    progress.enable_steady_tick(Duration::from_millis(100));

    if ids.len() > 1 && version.is_some() {
        println!("Only use the --version flag when adding a single mod");
        return Ok(());
    }

    let mut mods = Vec::new();
    let mut invalid_ids = Vec::new();

    for (idx, id) in ids.iter().enumerate() {
        progress.set_message(format!("Adding mods {}/{}", idx + 1, ids.len()));

        let res_mod = match get_mod(id, &version, &modpack).await {
            Ok(res) => res,
            Err(err) => {
                if matches!(err, Error::InvalidId) {
                    invalid_ids.push(id.to_owned());
                    continue;
                } else {
                    return Err(err);
                }
            },
        };

        mods.push(res_mod);
    }

    // search for ids that came back invalid on modrinth
    // and add selected ones
    let search_mods = progress.suspend(|| async {
        search_mods(invalid_ids, &modpack).await
    }).await?;

    mods.extend(search_mods);

    progress.set_message("Finding dependencies");

    let mut tasks: JoinSet<crate::Result<()>> = JoinSet::new();

    // use tokio::sync::Mutex to hold MutexGuard across .await
    let dependencies = Arc::new(Mutex::new(Vec::new()));
    for m in mods.clone() {
        let modpack = modpack.clone();
        let dependencies = dependencies.clone();

        let task = async move {
            get_dependencies(&m, &dependencies, &modpack).await?;
            Ok(())
        };

        tasks.spawn(task);
    }

    join_all(tasks).await?;
    let mut dependencies = dependencies.lock().await.clone();
    println!("{:#?}", dependencies);

    // remove dependencies already present in index
    // to not show "already in modpack" for dependencies
    let index_mods = Index::read()?.mods;
    dependencies.retain(|d| !index_mods.contains(d));
    
    mods.extend(dependencies);

    progress.finish_and_clear();
    add_mods_to_index(mods).await?;
    Ok(())
}

pub async fn add_mods_to_index(mods: Vec<Mod>) -> Result<()> {
    let mut index = Index::read()?;
    for m in mods {
        // checking the name as well so you cant add the same mod from both modrinth or curseforge
        if index.mods.iter().any(|idx_mod| idx_mod.name == m.name || *idx_mod.id == m.id) {
            println!("{} is already in the modpack!", m.name);
            continue;
        }
        println!("Adding {}", m.name);
        index.mods.push(m)
    }

    index.mods.sort_by_key(|m| m.name.to_owned());
    Index::write(&index)?;
    Ok(())
}

async fn get_mod(id: &str, version: &Option<String>, modpack: &Modpack) -> Result<Mod> {
    // ids parseable as i32 are curseforge mods
    let result = match id.parse::<i32>() {
        Ok(id) => {
            let (cf_mod, file) = if version.is_none() {
                let (cf_mod, files) = try_join!(
                    CURSEFORGE.get_mod(id),
                    CURSEFORGE.get_mod_files(id)
                )?;

                let project_type = ProjectType::try_from(cf_mod.class_id.unwrap())?;

                let compatibles = files.into_iter().filter(|f| 
                    f.is_available
                    && if matches!(project_type, ProjectType::Mod) { f.game_versions.contains(&modpack.versions.mod_loader.to_string()) } else { true }
                    && f.game_versions.contains(&modpack.versions.minecraft)
                ).collect::<Vec<File>>();

                if compatibles.is_empty() {
                    return Err(Error::Other(format!("No compatible versions found for mod: '{}'", cf_mod.name)));
                }

                (cf_mod, compatibles.into_iter().max_by_key(|f| f.file_date).unwrap())
            } else {
                try_join!(
                    CURSEFORGE.get_mod(id),
                    CURSEFORGE.get_mod_file(id, version.as_ref().unwrap().parse::<i32>().unwrap())
                )?
            };

            let project_type = ProjectType::try_from(cf_mod.class_id.unwrap())?;
    
            #[allow(clippy::unnecessary_operation)]
            Mod {
                name: cf_mod.name,
                project_type,
                platform: ModPlatform::CurseForge,
                id: cf_mod.id.to_string(),
                version: file.id.to_string(),
                pinned: false,
            }
        },
        Err(_) => {
            let (mr_mod, version) = if version.is_none() {
                // using list_versions_filtered requires me to know whether the project is a mod
                // filter later to allow the 2 requests to be joined
                let (mr_mod, versions) = try_join!(
                    MODRINTH.get_project(id),
                    MODRINTH.list_versions(id)
                ).map_err(|_| Error::InvalidId)?;

                match mr_mod.project_type {
                    MRProjectType::Modpack | MRProjectType::Plugin => {
                        return Err(Error::Other(format!("Unable to add {} becaue its project type is unsupported", mr_mod.title)));
                    },
                    _ => {}
                }

                let compatibles: Vec<Version> = versions.into_iter().filter(|v|
                    v.game_versions.contains(&modpack.versions.minecraft)
                    && if matches!(mr_mod.project_type, MRProjectType::Mod) { v.loaders.contains(&modpack.versions.mod_loader.to_string().to_lowercase()) } else { true }
                ).collect();

                if compatibles.is_empty() {
                    return Err(Error::Other(format!("No compatible versions found for mod: '{}'", mr_mod.title)));
                }

                (mr_mod, compatibles.into_iter().max_by_key(|v| v.date_published).unwrap())
            } else {
                let version = version.as_ref().unwrap();
                try_join!(
                    MODRINTH.get_project(id),
                    MODRINTH.get_version(version)
                )?
            };

            Mod {
                name: mr_mod.title,
                project_type: mr_mod.project_type.try_into()?,
                platform: ModPlatform::Modrinth,
                id: mr_mod.id,
                version: version.id,
                pinned: false,
            }
        },
    };
    
    Ok(result)
}

async fn search_mods(ids: Vec<String>, modpack: &Modpack) -> Result<Vec<Mod>> {
    let mut results = Vec::new();

    let search_facets = vec![
        vec![Facet::ProjectType(MRProjectType::Mod)],
        vec![Facet::Versions(modpack.versions.minecraft.clone())],
        vec![Facet::Categories(modpack.versions.mod_loader.to_string().to_lowercase().clone())] // loaders are packed together with categories
    ];

    for id in ids {
        let search_res = MODRINTH.search_paged(
            &id,
            &Sort::Relevance,
            15, // limit
            0, // offset
            search_facets.clone()
        ).await?;

        if search_res.hits.is_empty() {
            println!("Searching for '{id}' returned nothing");
            continue;
        }

        let search_titles: Vec<&String> = search_res.hits.iter().map(|hit| &hit.title).collect();

        // if inputed string matches a search result title exactly choose that one
        let chosen = if let Some(exact_match) = search_titles.iter().position(|t| t.to_lowercase() == id.to_lowercase()) {
            exact_match
        } else {
            let selected = Select::new()
                    .with_prompt(format!("suggestions for '{id}'"))
                    .items(&search_titles)
                    .interact_opt()?;

            match selected {
                Some(usize) => usize,
                None => continue, // selecting is cancellable
            }
        };

        let chosen_id = &search_res.hits[chosen].project_id;
        results.push(get_mod(chosen_id, &None, modpack).await?);
    };

    Ok(results)
}

#[async_recursion]
async fn get_dependencies(idx_mod: &Mod, dependencies: &Arc<Mutex<Vec<Mod>>>, modpack: &Modpack) -> Result<()> {
    match idx_mod.platform {
        ModPlatform::Modrinth => {
            let mod_dependencies = MODRINTH.get_version(&idx_mod.version).await?.dependencies;
            let required_dependencies: Vec<Dependency> = mod_dependencies.into_iter().filter(|d| matches!(d.dependency_type, DependencyType::Required)).collect();
            for dep in required_dependencies {
                if dep.project_id.is_none() {
                    continue;
                }

                // avoid rechecking a dependency multiple times
                let mut dependencies_lock = dependencies.lock().await;
                if !dependencies_lock.iter().any(|m| &m.id == dep.project_id.as_ref().unwrap()) {
                    // modrinth optionally provides version ids on dependencies so use those
                    let mr_mod = get_mod(&dep.project_id.unwrap(), &dep.version_id, modpack).await?;
                    dependencies_lock.push(mr_mod.to_owned());
                    drop(dependencies_lock);

                    get_dependencies(&mr_mod, dependencies, modpack).await?;
                }
            }
        },
        ModPlatform::CurseForge => {
            let file = CURSEFORGE.get_mod_file(
            idx_mod.id.parse::<i32>().unwrap(),
            idx_mod.version.parse::<i32>().unwrap()
            ).await?;

            let mod_dependencies: Vec<FileDependency> = file.dependencies.into_iter().filter(|f| matches!(f.relation_type, FileRelationType::RequiredDependency)).collect();

            for dep in mod_dependencies {
                // avoid rechecking a dependency multiple times
                let mut dependencies_lock = dependencies.lock().await;
                if !dependencies_lock.iter().any(|m| m.id == dep.mod_id.to_string()) {
                    let cf_mod = get_mod(&dep.mod_id.to_string(), &None, modpack).await?;
                    dependencies_lock.push(cf_mod.to_owned());
                    drop(dependencies_lock);

                    get_dependencies(&cf_mod, dependencies, modpack).await?;
                }
            }

        },
    }

    Ok(())
}