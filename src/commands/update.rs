use std::{collections::HashMap, fmt::Write, sync::Arc};

use console::style;
use lazy_regex::Regex;
use supports_hyperlinks::supports_hyperlinks;
use tokio::{task::JoinSet, try_join};

use crate::{
    api::{curseforge::File, github::GithubRelease, modrinth::Version}, error::Result, structs::{
        index::{Addon, AddonSource, CurseforgeSource, GithubSource, Index, ModrinthSource},
        pack::Modpack,
    }, util::modrinth::get_primary_hash, CURSEFORGE, GITHUB, MODRINTH
};

pub async fn update() -> Result<()> {
    let modpack = Arc::new(Modpack::read()?);
    let mut index = Index::read().await?;
    // filter out pinned mods so they dont get updated
    index.addons.retain(|a| !a.options.as_ref().is_some_and(|a| a.pinned));

    let mut mr_addon_versions = Vec::new();
    let mut cf_addon_ids = Vec::new();
    let mut gh_addon_sources = Vec::new(); // (gh_repo, tag_filter, title_filter)

    index.addons.iter().for_each(|a| match &a.source {
        AddonSource::Modrinth(source) => mr_addon_versions.push(source.version.as_str()),
        AddonSource::Curseforge(source) => cf_addon_ids.push(source.id),
        AddonSource::Github(source) => gh_addon_sources.push((source.repo.clone(), source.tag_filter.clone(), source.title_filter.clone()))
    });

    let (
        latest_mr_versions,
        (latest_cf_versions, cf_links),
        latest_gh_versions
    ) = try_join!(
        update_modrinth(&modpack, mr_addon_versions),
        update_curseforge(&modpack, cf_addon_ids),
        update_github(gh_addon_sources)
    )?;

    // Mods with updated version ids
    // (Addon, new_version_url/version_name)
    let to_update: Vec<(Addon, String)> = index.addons.into_iter().filter_map(|addon| {
        match &addon.source {
            AddonSource::Modrinth(source) => {
                let latest_version = latest_mr_versions.values().find(|v| v.project_id == source.id).unwrap();
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

// using https://crates.io/crates/supports-hyperlinks
// to test if hyperlinks in terminal are supported and use a link if they are
fn to_hyperlink(link: &str, placeholder: &str) -> String {
    if supports_hyperlinks() {
        format!("\u{1b}]8;;{}\u{1b}\\{}\u{1b}]8;;\u{1b}\\", link, placeholder)
    } else {
        placeholder.into()
    }
}

async fn update_modrinth(modpack: &Modpack, mr_addon_versions: Vec<&str>) -> Result<HashMap<String, Version>> {
    let mr_version_hashes: Vec<String> = MODRINTH
        .get_versions(mr_addon_versions.as_slice())
        .await?
        .into_iter()
        .map(|v| get_primary_hash(v.files).expect("couldnt find hash"))
        .collect();

    let loader_string = modpack.versions.loader.to_string().to_lowercase();

    MODRINTH
        .latest_versions_from_hashes(
            mr_version_hashes
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<&str>>()
                .as_slice(),
            // include these for: shader, datapack, resourcepack support
            Some(&[loader_string.as_str(), "iris", "canvas", "optifine", "datapack", "minecraft"]),
            Some(&[&*modpack.versions.minecraft]),
        )
        .await
}

async fn update_curseforge(modpack: &Modpack, cf_addon_ids: Vec<i32>) -> Result<(Vec<File>, Vec<(i32, String)>)> {
    let mut tasks: JoinSet<Result<Option<File>>> = JoinSet::new();
    for id in cf_addon_ids.clone() {
        let modpack = modpack.clone();

        let task = async move {
            let files = CURSEFORGE.get_mod_files(&id).await?;

            let compatibles = files.into_iter().filter(|f| 
                    f.is_available
                    && f.game_versions.contains(&modpack.versions.loader.to_string())
                    && f.game_versions.contains(&modpack.versions.minecraft)
                ).collect::<Vec<File>>();
    
            Ok(compatibles.first().map(|c| c.to_owned()))
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
    let cf_links: Vec<(i32, String)> = CURSEFORGE.get_mods(cf_addon_ids).await.unwrap_or_default().into_iter().map(|a| (a.id, a.links.website_url)).collect();

    Ok((latest_cf_versions, cf_links))
}

async fn update_github(gh_addon_sources: Vec<(String, Option<String>, Option<String>)>) -> Result<Vec<(String, GithubRelease)>> {
    // pair of repo & latest compatible release
    let mut tasks: JoinSet<Result<(String, Option<GithubRelease>)>> = JoinSet::new();
    for (repo, tag_filter, title_filter) in gh_addon_sources {
        let task = async move {
            let repo_split: Vec<&str> = repo.split('/').collect();
            let mut releases = GITHUB.list_releases(repo_split[0], repo_split[1]).await?;
            gh_apply_filter(&mut releases, tag_filter.clone(), FilterType::Tag);
            gh_apply_filter(&mut releases, title_filter.clone(), FilterType::Title);
    
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

fn gh_apply_filter(releases: &mut Vec<GithubRelease>, filter: Option<String>, filter_type: FilterType) {
    if filter.is_none() {
        return;
    }

    match Regex::new(filter.as_ref().unwrap()) {
        Ok(regex) => {
            releases.retain(|r| regex.is_match(match filter_type {
                FilterType::Tag => &r.tag_name,
                FilterType::Title => &r.name,
            }))
        },
        Err(_) => println!("{} is not a valid regex pattern, skipping applying on github releases", filter.unwrap()),
    }
}

enum FilterType {
    Tag,
    Title
}