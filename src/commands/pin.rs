use console::style;

use crate::{cli::PinArgs, error::{Error, Result}, structs::index::{Addon, AddonOptions, AddonSource, CurseforgeSource, GithubSource, Index, ModrinthSource}, CURSEFORGE, GITHUB, MODRINTH};

pub async fn pin(args: PinArgs) -> Result<()> {
    let index = Index::read().await?;

    if let Some(addon) = index.select_addon(&args.addon).cloned() {
        if !addon.options.clone().unwrap_or_default().pinned {
            if let Some(version) = &args.version {
                check_version(&addon, version).await?;
            }

            println!("Pinning {} {}", addon.name, style(args.version.clone().unwrap_or_default()).dim());
            
            let addon = Addon {
                options: Some(AddonOptions {
                    pinned: true,
                    ..addon.options.unwrap_or_default()
                }),
                source: match addon.source {
                    AddonSource::Modrinth(source) => AddonSource::Modrinth(
                        ModrinthSource { version: args.version.unwrap_or(source.version), ..source }
                    ),
                    AddonSource::Curseforge(source) => AddonSource::Curseforge(
                        CurseforgeSource { version: args.version.map(|v| v.parse::<i32>().unwrap()).unwrap_or(source.version), ..source }
                    ),
                    AddonSource::Github(source) => AddonSource::Github(
                        GithubSource { repo: args.version.unwrap_or(source.repo), ..source }
                    ),
                },
                ..addon
            };

            Index::write_addons(vec![addon]).await?;
        } else {
            println!("{}", style(format!("{} is already pinned", addon.name)).color256(166))
        }
    }

    Ok(())
}

// basically just a sanity check for if the version exists & is a version of the addon selected
async fn check_version(addon: &Addon, version: &str) -> Result<()> {
    let compatible = match &addon.source {
        AddonSource::Modrinth(source) => {
            MODRINTH.get_version(version).await.is_ok_and(|version| version.project_id == source.id)
        },
        AddonSource::Curseforge(source) => {
            if let Ok(version) = version.parse::<i32>() {
                CURSEFORGE.get_mod_file(&source.id, &version).await.is_ok()
            } else {
                return Err(Error::Other("Could not parse version id to integer".into()));
            }

        },
        AddonSource::Github(source) => {
            let repo_split: Vec<&str> = source.repo.split('/').collect();
            GITHUB.get_release_by_tag(repo_split[0], repo_split[1], version).await.is_ok()
        },
    };

    if !compatible {
        return Err(Error::Other(format!("Could not find version with id '{}' on {}", version, addon.name)));
    }

    Ok(())
}