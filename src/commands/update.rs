use std::{fmt::Write, sync::Arc};

use tokio::task::JoinSet;

use crate::{
    api::{curseforge::File, github::GithubRelease}, error::Result, structs::{
        index::{Addon, AddonSource, CurseforgeSource, GithubSource, Index, ModrinthSource, ReleaseFilter},
        pack::Modpack,
    }, util::modrinth::get_primary_hash, CURSEFORGE, GITHUB, MODRINTH
};

pub async fn update() -> Result<()> {
    let modpack = Arc::new(Modpack::read()?);
    let mut index = Index::read().await?;
    // filter out pinned mods so they dont get updated
    index.addons.retain(|a| !a.options.as_ref().is_some_and(|a| a.pinned));

    // modrinth updates

    let mr_addon_versions: Vec<&str> = index
        .addons
        .iter()
        .filter_map(|a| match &a.source {
            AddonSource::Modrinth(source) => Some(source.version.as_str()),
            _ => None,
        })
        .collect();

    let mr_version_hashes: Vec<String> = MODRINTH
        .get_versions(mr_addon_versions.as_slice())
        .await?
        .into_iter()
        .map(|v| get_primary_hash(v.files).expect("couldnt find hash"))
        .collect();

    let loader_string = modpack.versions.loader.to_string().to_lowercase();
    let latest_mr_versions = MODRINTH
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
        .await?;

    // curseforge updates

    let cf_addon_ids: Vec<i32> = index.addons.iter().filter_map(|a| match &a.source {
        AddonSource::Curseforge(source) => Some(source.id),
        _ => None
    }).collect();

    let mut tasks: JoinSet<Result<Option<File>>> = JoinSet::new();
    for id in cf_addon_ids {
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

    // github updates

    // pair of repo string & release filter
    let github_sources: Vec<(String, ReleaseFilter)> = index.addons.iter().filter_map(|a| match &a.source {
        AddonSource::Github(source) => Some((source.repo.clone(), source.filter_by.clone())),
        _ => None
    }).collect();

    // pair of repo & latest compatible release
    let mut tasks: JoinSet<Result<(String, Option<GithubRelease>)>> = JoinSet::new();
    for (repo, filter) in github_sources {
        let modpack = modpack.clone();

        let task = async move {
            let repo_split: Vec<&str> = repo.split('/').collect();
            let mut releases = GITHUB.list_releases(repo_split[0], repo_split[1]).await?;
            releases.retain(|r| match filter {
                ReleaseFilter::Tag => r.tag_name.contains(&modpack.versions.minecraft),
                ReleaseFilter::Title => r.name.contains(&modpack.versions.minecraft),
                ReleaseFilter::None => true,
            });
    
            Ok((repo, releases.first().map(|r| r.to_owned())))
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

    // Mods with updated version ids
    let to_update: Vec<Addon> = index.addons.into_iter().filter_map(|addon| {
        match &addon.source {
            AddonSource::Modrinth(source) => {
                let latest_version = latest_mr_versions.values().find(|v| v.project_id == source.id).unwrap();
                if latest_version.id != source.version {
                    return Some(Addon { source: AddonSource::Modrinth(ModrinthSource { id: source.id.clone(), version: latest_version.id.clone() }), ..addon });
                }
            },
            AddonSource::Curseforge(source) => {
                let latest_version = latest_cf_versions.iter().find(|v| v.mod_id == source.id).unwrap();
                if latest_version.id != source.version {
                    return Some(Addon { source: AddonSource::Curseforge(CurseforgeSource { id: source.id, version: latest_version.id }), ..addon });
                }
            },
            AddonSource::Github(source) => {
                let latest_version = latest_gh_versions.iter().find(|r| r.0 == source.repo).unwrap();
                if latest_version.1.tag_name != source.tag {
                    return Some(Addon { source: AddonSource::Github(GithubSource { tag: latest_version.1.tag_name.clone(), ..source.clone() }), ..addon});
                }
            },
        }

        None
    }).collect();

    if to_update.is_empty() {
        println!("No new updates found!");
    } else {
        println!(
            "{}",
            to_update.iter().fold(String::new(), |mut out, a| {
                write!(out, "Updating {}", a.name).unwrap();
                out
            })
        );
        
        Index::write_addons(to_update).await?;
    }

    Ok(())
}
