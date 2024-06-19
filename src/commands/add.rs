use std::time::Duration;
use dialoguer::Select;
use ferinth::{check_id_slug, structures::{project::{Project, ProjectType}, search::{Facet, Sort}}};
use furse::structures::file_structs::File;
use indicatif::ProgressBar;

use crate::{structs::{Index, Mod, Modpack}, Error, Result, CURSEFORGE, MODRINTH};

pub async fn add_mod(mods: Vec<String>, ignore_version: bool, ignore_loader: bool) -> Result<()> {
    let modpack = Modpack::read()?;
    let progress = ProgressBar::new_spinner().with_message("Adding mods");
    progress.enable_steady_tick(Duration::from_millis(100));

    let mut to_add: Vec<Mod> = Vec::new();

    let mods_len = mods.len();
    for (idx, id) in mods.into_iter().enumerate() {
        if mods_len > 1 {
            progress.set_message(format!("Adding mods {}/{}", idx, mods_len));
        }

        // handle curseforge mods
        // curseforge ids are always i32
        if let Ok(id) = id.parse::<i32>() {
            // File structure doesnt contain the mod name so we need to check that seperately..
            let cf_mod = CURSEFORGE.get_mod(id).await?;
                    
            // 432 == minecraft
            if cf_mod.game_id != 432 { 
                return Err(Error::Other(format!("{} is not a minecraft mod silly!", cf_mod.name)));
            }
        
            let files = CURSEFORGE.get_mod_files(cf_mod.id).await?;
    
            let compatibles = files.into_iter().filter(|f| 
                    f.is_available
                    && if !ignore_loader { f.game_versions.contains(&modpack.versions.mod_loader.to_string()) } else { true }
                    && if !ignore_version { f.game_versions.contains(&modpack.versions.minecraft) } else { true }
                ).collect::<Vec<File>>();
            
            if compatibles.is_empty() {
                return Err(Error::Other(format!("No compatible versions for mod with id: '{}' on curseforge", cf_mod.name)));
            }

            let latest_file = compatibles.into_iter().max_by_key(|f| f.file_date).unwrap();
            to_add.push(Mod::new(cf_mod.name.to_owned(), None, Some(latest_file.mod_id), latest_file.id.to_string(), false));
            continue;
        }

        // handle modrinth mods
        let mr_mod = match get_project_with_search(&id, &modpack, ignore_version, ignore_loader, &progress).await? {
            Some(m) => m,
            None => continue,
        };

        // I honestly dont know how &[&str] works so-
        let loader_slice = &[&*modpack.versions.mod_loader.to_string().to_lowercase()];
        let version_slice = &[&*modpack.versions.minecraft];

        let compatible_versions = MODRINTH.list_versions_filtered(
                &mr_mod.id,
                if !ignore_loader { Some(loader_slice) } else { None },
                if !ignore_version { Some(version_slice) } else { None },
                None,
            )
            .await?;

        if compatible_versions.is_empty() {
            progress.println(format!("{} has no compatible versions!", mr_mod.title));
            continue;
        }

        // get version with latest publish date
        let latest_compatible_version = compatible_versions.into_iter().max_by_key(|v| v.date_published).unwrap();
        let primary_file = latest_compatible_version.files.into_iter().find(|f| f.primary).unwrap();
        
        to_add.push(Mod::new(mr_mod.title, Some(mr_mod.id), None, primary_file.hashes.sha1, false));
    }

    progress.finish_and_clear();
    add_mods(to_add)?;
    Ok(())
}

pub fn add_mods(mods: Vec<Mod>) -> Result<()> {
    let mut index = Index::read()?;
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
    Index::write(&index)?;
    Ok(())
}

// try to get a modrinth project from id/slug string
// & suggest search results if the id/slug cant be found
async fn get_project_with_search(id: &str, modpack: &Modpack, ignore_version: bool, ignore_loader: bool, progress: &ProgressBar) -> Result<Option<Project>> {
    if let Ok(project) = valid_id_slug_helper(id).await  {
        Ok(Some(project))
    }
    else {
        let mut search_facets: Vec<Vec<Facet>> = Vec::new();
        search_facets.push(vec![Facet::ProjectType(ProjectType::Mod)]);

        if !ignore_version {
            search_facets.push(vec![Facet::Versions(modpack.versions.minecraft.to_owned())])
        }
        if !ignore_loader {
            search_facets.push(vec![Facet::Categories(modpack.versions.mod_loader.to_string())])
        }

        let search_res = MODRINTH.search_paged(
            id,
            &Sort::Relevance,
            10,
            0,
            search_facets
        ).await?;

        if search_res.hits.is_empty() {
            progress.println(format!("Searching for '{id}' returned nothing"));
            return Ok(None);
        }

        let search_titles: Vec<&String> = search_res.hits.iter().map(|hit| &hit.title).collect();

        // if inputed string matches a search result title exactly choose that one
        let chosen = if let Some(exact_match) = search_titles.iter().position(|t| t.to_lowercase() == id.to_lowercase()) {
            exact_match
        } else {
            let selection = progress.suspend(|| {
                Select::new()
                    .with_prompt(format!("suggestions for '{id}'"))
                    .items(&search_titles)
                    .clear(true)
                    .interact_opt().unwrap()
            });

            match selection {
                Some(choice) => choice,
                None => return Ok(None), // selecting a suggestion is optional, return Ok(None) if none are selected
            }
        };
            
        let chosen_project = MODRINTH.get_project(&search_res.hits[chosen].project_id).await?;
        Ok(Some(chosen_project))
    }
}

// if check_id_slug(&[id]).is_ok() && let Ok(project) = MODRINTH.get_project(id).await
// https://github.com/rust-lang/rust/issues/53667
async fn valid_id_slug_helper(id: &str) -> Result<Project> {
    check_id_slug(&[id])?;
    Ok(MODRINTH.get_project(id).await?)
}