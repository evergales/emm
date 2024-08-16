use std::fmt::Write;

use console::style;

use crate::{cli::ListArgs, error::Result, structs::index::{Addon, AddonSource, Index}, util::to_hyperlink, CURSEFORGE, GITHUB, MODRINTH};

pub async fn list(args: ListArgs) -> Result<()> {
    let mut index = Index::read().await?;
    index.addons.sort_by_key(|a| a.name.clone());

    let mut out = String::new();

    let cf_addon_ids = index.addons.iter().filter_map(|a| match &a.source {
        AddonSource::Curseforge(source) => Some(source.id),
        _ => None
    }).collect();

    let cf_links: Vec<(i32, String)> = CURSEFORGE.get_mods(cf_addon_ids).await.unwrap_or_default().into_iter().map(|a| (a.id, a.links.website_url)).collect();

    if args.markdown {
        let mut mr_ids = Vec::new();
        let mut cf_ids = Vec::new();
        let mut gh_ids = Vec::new();
        index.addons.iter().for_each(|a| match &a.source {
            AddonSource::Modrinth(source) => mr_ids.push(source.id.as_str()),
            AddonSource::Curseforge(source) => cf_ids.push(source.id),
            AddonSource::Github(source) => gh_ids.push({
                let repo_split: Vec<&str> = source.repo.split('/').collect();
                (repo_split[0].to_owned(), repo_split[1].to_owned())
            }),
        });

        // (project_id, description)
        let mut descriptions: Vec<(String, String)> = Vec::new();
        descriptions.extend(MODRINTH.get_multiple_projects(&mr_ids).await?.into_iter().map(|a| (a.id, a.description)));
        descriptions.extend(CURSEFORGE.get_mods(cf_ids).await?.into_iter().map(|a| (a.id.to_string(), a.summary)));
        descriptions.extend(GITHUB.get_repos(gh_ids).await?.into_iter().map(|a| (a.full_name.to_lowercase(), a.description)));
        

        for addon in index.addons {
            let url = get_url(&addon, &cf_links);
            let version_url = match &addon.source {
                AddonSource::Modrinth(source) => format!("{url}/version/{}", source.version),
                AddonSource::Curseforge(source) => format!("{url}/files/{}", source.version),
                AddonSource::Github(source) => format!("{url}/releases/tag/{}", source.tag),
            };

            writeln!(&mut out, "**[{name}]({url})** ([version]({version_url}))\n{description}\n",
                name = addon.name,
                description = descriptions.iter().find(|d| d.0 == addon.generic_id()).unwrap().1
            ).unwrap()
        }
    } else {
        let max_name_width = index.addons.iter().max_by_key(|a| a.name.len()).unwrap().name.len();

        for addon in index.addons {
            let id_prefix = match addon.source {
                AddonSource::Modrinth(_) => style("MR").green().dim(),
                AddonSource::Curseforge(_) => style("CF").color256(166).dim(),
                AddonSource::Github(_) => style("GH").magenta().dim(),
            };

            let url = get_url(&addon, &cf_links);

            writeln!(&mut out,
                "{name:max_name_width$}  {id_prefix} {id_link}",
                name = style(&addon.name).bold(),
                id_link = style(to_hyperlink(&url, &addon.generic_id())).dim()
            ).unwrap();
        }
    }

    print!("{}", out);
    Ok(())
}

fn get_url(addon: &Addon, cf_links: &[(i32, String)]) -> String {
    match &addon.source {
        AddonSource::Modrinth(source) => format!("https://modrinth.com/project/{}", source.id),
        AddonSource::Curseforge(source) => cf_links.iter().find(|l| l.0 == source.id).unwrap().1.clone(),
        AddonSource::Github(source) => format!("https://github.com/{}", source.repo),
    }
}