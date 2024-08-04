use console::style;
use dialoguer::{Confirm, Select};
use lazy_regex::regex_captures;

use crate::{api::github::GithubRelease, error::{Error, Result}, structs::{index::{Addon, AddonOptions, AddonSource, GithubSource, Index, ProjectType, ReleaseFilter}, pack::Modpack}, util::github::find_filter, GITHUB};

pub async fn add_github(repo_input: String, tag: Option<String>, first_asset: bool) -> Result<()> {
    let modpack = Modpack::read()?;

    // regex to extract user & repo
    // accepts github urls and just "user/repo"
    let (user, repo) = match regex_captures!(r#"(?:https?:\/\/github\.com\/)?([\w.-]+?)\/([\w.-]+)(?:\/.*)?"#, &repo_input) {
        Some((_, user, repo)) => (user, repo),
        None => return Err(Error::Other(format!("{} \nuse a github url or user/repo'", style("Invalid github url").color256(166)))),
    };

    let releases = GITHUB.list_releases(user, repo).await?;
    let release_names: Vec<&str> = releases.iter().map(|r| r.name.as_str()).collect();
    let tags: Vec<&str> = releases.iter().map(|r| r.tag_name.as_str()).collect();

    let tag_filter = tags.iter().find(|t| find_filter(t, &modpack).is_some());
    let title_filter = release_names.iter().find(|t| find_filter(t, &modpack).is_some());

    let (mut filter_by, filter) = if let Some(found) = tag_filter {
        (ReleaseFilter::Tag, Some(find_filter(tags.iter().find(|t| t == &found).unwrap(), &modpack).unwrap()))
    } else if let Some(found) = title_filter {
        (ReleaseFilter::Title, Some(find_filter(release_names.iter().find(|n| n == &found).unwrap(), &modpack).unwrap()))
    } else {
        (ReleaseFilter::None, None)
    };

    if filter_by != ReleaseFilter::None {
        println!("filter releases by {}: {}", filter_by, filter.unwrap());
        if !Confirm::new()
            .with_prompt("Use filter")
            .interact()
            .unwrap()
        {
            filter_by = ReleaseFilter::None
        }
    }

    let release = if tag.is_some() || filter_by == ReleaseFilter::None {
        match tag {
            Some(tag) => {
                match releases.iter().find(|r| r.tag_name == tag) {
                    Some(release) => release.clone(),
                    None => return Err(Error::Other(format!("Could not find a release tagged '{tag}'"))),
                }
            },
            None => {
                let idx = Select::new()
                    .with_prompt("Select a release")
                    .items(&release_names)
                    .interact()
                    .unwrap();
    
                releases[idx].clone()
            },
        }
    } else {
        let filtered: Vec<GithubRelease> = releases.clone().into_iter().filter(|r| {
            match filter_by {
                ReleaseFilter::Tag => r.tag_name.to_lowercase().contains(&modpack.versions.minecraft),
                ReleaseFilter::Title => r.name.to_lowercase().contains(&modpack.versions.minecraft),
               _ => unreachable!()
            }
        }).collect();

        filtered.first().unwrap().clone()
    };

    let asset_index =
    if first_asset || release.assets.len() == 1 {
        0 
    } else {
        let asset_names: Vec<&str> = release.assets.iter().map(|a| a.name.as_str()).collect();
        Select::new()
            .with_prompt("Select a release asset")
            .items(&asset_names)
            .interact()
            .unwrap()
    };

    let addon = Addon {
        name: repo.to_owned(),
        project_type: ProjectType::Unknown,
        source: AddonSource::Github(GithubSource {
            repo: format!("{user}/{repo}"),
            tag: release.tag_name,
            filter_by,
            asset_index,
        }),
        options: Some(AddonOptions::default()),
        filename: None
    };

    println!("Adding {repo}");
    Index::write_addons(vec![addon]).await?;
    Ok(())
}
