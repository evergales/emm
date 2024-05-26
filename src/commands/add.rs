use dialoguer::Select;
use ferinth::{check_id_slug, structures::{project::{Project, ProjectType}, search::Facet}};
use furse::structures::file_structs::File;

use crate::{structs::{Index, Mod, Modpack}, Error, Result, CURSEFORGE, MODRINTH};

pub async fn add_mod(mods: Vec<String>) -> Result<()> {
    let modpack = Modpack::read()?;

    let mut to_add: Vec<Mod> = Vec::new();

    for id in mods {
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
                    && f.game_versions.contains(&modpack.mod_loader.to_string())
                    && f.game_versions.contains(&modpack.game_version)
                ).collect::<Vec<File>>();
            
            if compatibles.is_empty() {
                return Err(Error::Other(format!("No compatible versions for mod with id: '{}' on curseforge", cf_mod.name)));
            }

            let latest_file = compatibles.into_iter().max_by_key(|f| f.file_date).unwrap();
            to_add.push(Mod::new(cf_mod.name.to_owned(), None, Some(latest_file.mod_id), latest_file.id.to_string(), None));
            continue;
        }

        // handle modrinth mods
        let mr_mod = match get_project_with_search(&id, &modpack).await? {
            Some(m) => m,
            None => continue,
        };

        let compatible_versions = MODRINTH.list_versions_filtered(
                &mr_mod.id,
                Some(&[&modpack.mod_loader.to_string().to_lowercase()]),
                Some(&[&modpack.game_version]),
                None,
            )
            .await?;

        if compatible_versions.is_empty() {
            eprintln!("{} has no compatible versions!", mr_mod.title);
            continue;
        }

        // get version with latest publish date
        let latest_compatible_version = compatible_versions.into_iter().max_by_key(|v| v.date_published).unwrap();
        let primary_file = latest_compatible_version.files.into_iter().find(|f| f.primary).unwrap();
        
        to_add.push(Mod::new(mr_mod.title, Some(mr_mod.id), None, primary_file.hashes.sha1, None))
    }

    Index::add_mods(to_add)?;
    Ok(())
}

// try to get a modrinth project from id/slug string
// & suggest search results if the id/slug cant be found
async fn get_project_with_search(id: &str, modpack: &Modpack) -> Result<Option<Project>> {
    if let Ok(project) = valid_id_slug_helper(id).await  {
        Ok(Some(project))
    }
    else {
        let search_res = MODRINTH
        .search(
            id,
            &ferinth::structures::search::Sort::Relevance,
            vec![vec![
                Facet::ProjectType(ProjectType::Mod),
                Facet::Categories(modpack.mod_loader.to_string()),
                Facet::Versions(modpack.game_version.to_owned())
            ]],
        )
        .await?;

        if search_res.hits.is_empty() {
            println!("Searching for '{id}' returned nothing");
            return Ok(None);
        }

        let search_titles: Vec<&String> = search_res.hits.iter().map(|hit| &hit.title).collect();
        
        let chosen = Select::new()
            .with_prompt(format!("suggestions for '{id}'"))
            .items(&search_titles)
            .interact_opt()?;
        
        // optional select will return none if exited
        if chosen.is_none() {
            return Ok(None);
        }
            
        let chosen_project = MODRINTH.get_project(&search_res.hits[chosen.unwrap()].project_id).await?;
        Ok(Some(chosen_project))
    }
}

// if check_id_slug(&[id]).is_ok() && let Ok(project) = MODRINTH.get_project(id).await
// https://github.com/rust-lang/rust/issues/53667
async fn valid_id_slug_helper(id: &str) -> Result<Project> {
    check_id_slug(&[id])?;
    Ok(MODRINTH.get_project(id).await?)
}