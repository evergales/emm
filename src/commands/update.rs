use std::{fmt::Write, sync::Arc};

use console::style;
use tokio::{task::JoinSet, try_join};

use crate::{
    api::{curseforge::File, github::GithubRelease, modrinth::Version}, cli::UpdateArgs, error::Result, structs::{
        index::{Addon, AddonSource, CurseforgeSource, GithubSource, Index, ModrinthSource, ProjectType},
        pack::Modpack,
    }, util::{to_hyperlink, FilterVersions}, CURSEFORGE, GITHUB, MODRINTH
};

pub async fn update(args: UpdateArgs) -> Result<()> {
    let modpack = Arc::new(Modpack::read()?);
    let mut index = Index::read().await?;

    // only use the addons in args if there are any
    if let Some(addons) = &args.addons {
        let selected_addons: Vec<Addon> = addons.iter().filter_map(|str| index.select_addon(str)).cloned().collect();
        index.addons = selected_addons
    }

    if args.addons.is_none() {
        // filter out pinned mods so they dont get updated
        index.addons.retain(|a| !a.options.as_ref().is_some_and(|a| a.pinned));
    }

    let mut mr_addon_sources = Vec::new();
    let mut cf_addon_sources = Vec::new();
    let mut gh_addon_sources = Vec::new();

    index.addons.iter().for_each(|a| match &a.source {
        AddonSource::Modrinth(source) => mr_addon_sources.push(source.id.as_str()),
        AddonSource::Curseforge(source) => cf_addon_sources.push((source.id, a.project_type.clone())),
        AddonSource::Github(source) => gh_addon_sources.push(source.repo.clone())
    });

    let (
        latest_mr_versions,
        (latest_cf_versions, cf_links),
        latest_gh_versions
    ) = try_join!(
        update_modrinth(&modpack, mr_addon_sources),
        update_curseforge(&modpack, cf_addon_sources),
        update_github(gh_addon_sources)
    )?;

    // Mods with updated version ids
    // (Addon, new_version_url/version_name)
    let to_update: Vec<(Addon, String)> = index.addons.into_iter().filter_map(|addon| {
        match &addon.source {
            AddonSource::Modrinth(source) => {
                let latest_version = &latest_mr_versions.iter().find(|v| v.0 == source.id).unwrap().1;
                if latest_version.id != source.version {
                    return Some((
                        Addon { source: AddonSource::Modrinth(ModrinthSource { id: source.id.clone(), version: latest_version.id.clone() }), ..addon },
                        to_hyperlink(&format!("https://modrinth.com/project/{}/version/{}", source.id, latest_version.id), &latest_version.version_number)
                    ));
                }
            },
            AddonSource::Curseforge(source) => {
                let latest_version = latest_cf_versions.iter().find(|v| v.mod_id == source.id).unwrap();
                if latest_version.id != source.version {
                    return Some((
                        Addon { source: AddonSource::Curseforge(CurseforgeSource { id: source.id, version: latest_version.id }), ..addon },
                        to_hyperlink(
                            &format!("{}/files/{}", cf_links.iter().find(|l| l.0 == source.id).unwrap().1, latest_version.id),
                            &latest_version.file_name
                        )
                    ));
                }
            },
            AddonSource::Github(source) => {
                let latest_version = &latest_gh_versions.iter().find(|r| r.0 == source.repo).unwrap().1;
                if latest_version.tag_name != source.tag {
                    return Some((
                        Addon { source: AddonSource::Github(GithubSource { tag: latest_version.tag_name.clone(), ..source.clone() }), ..addon},
                        to_hyperlink(&format!("https://github.com/{}/releases/tag/{}", source.repo, latest_version.tag_name), &latest_version.tag_name)
                    ));
                }
            },
        }

        None
    }).collect();

    if to_update.is_empty() {
        println!("No new updates found!");
    } else {
        println!(
            "Updating:{}",
            to_update.iter().fold(String::new(), |mut out, a| {
                write!(out, "\n{} {}", style(&a.0.name).bold(), style(&a.1).dim()).unwrap();
                out
            })
        );
        
        Index::write_addons(to_update.into_iter().map(|a| a.0).collect()).await?;
    }

    Ok(())
}

async fn update_modrinth(modpack: &Modpack, mr_addon_ids: Vec<&str>) -> Result<Vec<(String, Version)>> {
    let mr_projects = MODRINTH.get_multiple_projects_versions(&mr_addon_ids).await?;
    let project_version_pairs = mr_projects.into_iter().map(|(project, versions)| {
        let compatible_versions = versions.filter_compatible(modpack, &project.project_type);
        (project.id, compatible_versions.best_match(modpack).unwrap())
    }).collect();

    Ok(project_version_pairs)
}

async fn update_curseforge(modpack: &Modpack, cf_addon_ids: Vec<(i32, ProjectType)>) -> Result<(Vec<File>, Vec<(i32, String)>)> {
    if cf_addon_ids.is_empty() { return Ok(Default::default()); }
    let mut tasks: JoinSet<Result<Option<File>>> = JoinSet::new();
    for (id, project_type) in cf_addon_ids.clone() {
        let modpack = modpack.clone();

        let task = async move {
            let files = CURSEFORGE.get_mod_files(&id).await?;

            let compatibles = files.filter_compatible(&modpack, &project_type);
            Ok(compatibles.best_match(&modpack))
        };

        tasks.spawn(task);
    }

    let mut latest_cf_versions = Vec::new();
    while let Some(res) = tasks.join_next().await {
        // only push to latest versions if there are compatible versions
        if let Some(file) = res?? {
            latest_cf_versions.push(file);
        }
    }

    // (mod_id, website_url) for displaying links later
    let cf_links: Vec<(i32, String)> = CURSEFORGE.get_mods(cf_addon_ids.iter().map(|a| a.0).collect())
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|a| (a.id, a.links.website_url))
        .collect();

    Ok((latest_cf_versions, cf_links))
}

async fn update_github(gh_addon_sources: Vec<String>) -> Result<Vec<(String, GithubRelease)>> {
    if gh_addon_sources.is_empty() { return Ok(Default::default()); }
    // pair of repo & latest compatible release
    let mut tasks: JoinSet<Result<(String, Option<GithubRelease>)>> = JoinSet::new();
    for repo in gh_addon_sources {
        let task = async move {
            let repo_split: Vec<&str> = repo.split('/').collect();
            let releases = GITHUB.list_releases(repo_split[0], repo_split[1]).await?;
    
            Ok((repo.to_owned(), releases.first().map(|r| r.to_owned())))
        };

        tasks.spawn(task);
    }

    let mut latest_gh_versions: Vec<(String, GithubRelease)> = Vec::new();
    while let Some(res) = tasks.join_next().await {
        let res = res??;

        // res.1 (GithubRelease) will be None if no compatible versions are available
        if res.1.is_some() {
            latest_gh_versions.push((res.0, res.1.unwrap()));
        }
    }

    Ok(latest_gh_versions)
}