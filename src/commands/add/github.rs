use console::style;
use dialoguer::Select;
use lazy_regex::regex_captures;

use crate::{cli::AddGithubArgs, error::{Error, Result}, structs::index::{Addon, AddonOptions, AddonSource, GithubSource, Index, ProjectType, Side}, GITHUB};

pub async fn add_github(args: AddGithubArgs) -> Result<()> {
    // regex to extract user & repo
    // accepts github urls and just "user/repo"
    let (user, repo) = match regex_captures!(r#"(?:https?:\/\/github\.com\/)?([\w.-]+?)\/([\w.-]+)(?:\/.*)?"#, &args.repo) {
        Some((_, user, repo)) => (user, repo),
        None => return Err(Error::Other(format!("{} \nuse a github url or user/repo'", style("Invalid github url").color256(166)))),
    };

    let releases = GITHUB.list_releases(user, repo).await?;
    let release_names: Vec<&str> = releases.iter().map(|r| r.name.as_str()).collect();

    let release = match args.tag {
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
    };

    let asset_index =
    if args.first_asset || release.assets.len() == 1 {
        0 
    } else {
        let asset_names: Vec<&str> = release.assets.iter().map(|a| a.name.as_str()).collect();
        Select::new()
            .with_prompt("Select a release asset")
            .items(&asset_names)
            .interact()
            .unwrap()
    };

    // todo: Ask what type of addon the user's adding (things are exported to "overrides/unknown" rn..)

    let addon = Addon {
        name: repo.to_owned(),
        project_type: ProjectType::Unknown,
        side: Side::Both,
        source: AddonSource::Github(GithubSource {
            repo: format!("{user}/{repo}"),
            tag: release.tag_name,
            asset_index,
        }),
        options: Some(AddonOptions::default()),
        filename: None
    };

    println!("Adding {repo}");
    Index::write_addons(vec![addon]).await?;
    Ok(())
}
