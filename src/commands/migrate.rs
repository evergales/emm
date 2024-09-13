use std::{fmt::Write, ops::Deref, sync::Arc, time::Duration};

use console::style;
use dialoguer::Confirm;
use indicatif::ProgressBar;
use tokio::task::JoinSet;

use crate::{cli::MigrateArgs, commands::init::pick_game_version, error::Result, structs::{index::{Addon, AddonSource, CurseforgeSource, GithubSource, Index, ModrinthSource, ProjectType}, pack::{ModLoader, Modpack}}, util::versions::get_latest_loader_version, CURSEFORGE, MODRINTH};

enum Compatibility {
    Compatible,
    Partial, // compatible through acceptable_versions option
    Incompatible,
    Unknown // cant check compatibility
}

// (addon, new_addon_version, compatibility)
type AddonCompat = (Addon, Option<String>, Compatibility);

pub async fn migrate(args: MigrateArgs) -> Result<()> {
    let modpack = Arc::new(Modpack::read()?);
    let index = Index::read().await?;

    let new_version = Arc::new(pick_game_version(args.show_snapshots).await?);

    let progress = ProgressBar::new_spinner().with_message("Finding compatible versions");
    progress.enable_steady_tick(Duration::from_millis(100));

    let mut mr_addons = Vec::new();
    let mut cf_addons = Vec::new();
    let mut gh_addons = Vec::new();
    index.addons.into_iter().for_each(|a| match a.source.clone() {
        AddonSource::Modrinth(source) => mr_addons.push((a, source.id)),
        AddonSource::Curseforge(source) => cf_addons.push((a, source.id)),
        AddonSource::Github(_) => gh_addons.push(a)
    });

    let mut to_migrate: Vec<AddonCompat> = Vec::new();
    let mut tasks: JoinSet<Result<AddonCompat>> = JoinSet::new();

    for addon in mr_addons {
        let modpack = modpack.clone();
        let new_version = new_version.clone();
        let task = async move {
            let versions = MODRINTH.get_project_versions(&addon.1).await?;
    
            let (compatibility, version) = 'compat: {
                if !versions.iter().any(|v| match addon.0.project_type {
                    ProjectType::Mod => v.loaders.contains(&modpack.versions.loader.to_string().to_lowercase()),
                    _ => true // ignore loader checks if not a mod
                }) {
                    break 'compat (Compatibility::Incompatible, None);
                }
                
                if let Some(version) = versions.iter().find(|v| v.game_versions.contains(&new_version)) {
                    break 'compat (Compatibility::Compatible, Some(version.id.clone()));
                }
    
                if let Some(acceptable_versions) = &modpack.options.acceptable_versions {
                    if let Some(version) = versions.iter().find(|v| v.game_versions.iter().any(|v| acceptable_versions.contains(v))) {
                        break 'compat (Compatibility::Partial, Some(version.id.clone()));
                    }
                }
    
                (Compatibility::Incompatible, None)
            };

            Ok((addon.0, version, compatibility))
        };

        tasks.spawn(task);
    };
    
    for addon in cf_addons {
        let modpack = modpack.clone();
        let new_version = new_version.clone();
        let task = async move {
            let files = CURSEFORGE.get_mod_files(&addon.1).await?;
    
            let (compatibility, version) = 'compat: {
                if !files.iter().any(|f| match addon.0.project_type {
                    ProjectType::Mod => f.game_versions.contains(&modpack.versions.loader.to_string()),
                    _ => true // ignore loader checks if not a mod
                }) {
                    break 'compat (Compatibility::Incompatible, None);
                }
                
                if let Some(file) = files.iter().find(|f| f.game_versions.contains(&new_version)) {
                    break 'compat (Compatibility::Compatible, Some(file.id));
                }
    
                if let Some(acceptable_versions) = &modpack.options.acceptable_versions {
                    if let Some(file) = files.iter().find(|f| f.game_versions.iter().any(|v| acceptable_versions.contains(v))) {
                        break 'compat (Compatibility::Partial, Some(file.id));
                    }
                }
    
                (Compatibility::Incompatible, None)
            };

            Ok((addon.0, version.map(|v| v.to_string()), compatibility))
        };

        tasks.spawn(task);
    }

    while let Some(res) = tasks.join_next().await { to_migrate.push(res??) }

    to_migrate.extend(gh_addons.into_iter().map(|addon| (addon, None, Compatibility::Unknown)));

    progress.finish_and_clear();

    let compatible_count = to_migrate.iter().filter(|(_, _, c)| matches!(c, Compatibility::Compatible)).count();
    let partial_count = to_migrate.iter().filter(|(_, _, c)| matches!(c, Compatibility::Partial)).count();
    let incompatible_count = to_migrate.iter().filter(|(_, _, c)| matches!(c, Compatibility::Incompatible)).count();
    let unknown_count = to_migrate.iter().filter(|(_, _, c)| matches!(c, Compatibility::Unknown)).count();

    let mut out = String::new();

    for (addon, version, compatability) in &to_migrate {
        writeln!(&mut out, "{name} {version}",
            name = match compatability {
                Compatibility::Compatible => style(&addon.name).green(),
                Compatibility::Partial => style(&addon.name).color256(166),
                Compatibility::Incompatible => style(&addon.name).red(),
                Compatibility::Unknown => style(&addon.name).blue(),
            },
            version = style(format!("({})", version.as_ref().unwrap_or(&"?".into()))).dim()
        ).unwrap()
    };

    writeln!(&mut out, "\n{}
{compatible_count} {compatible} | {partial_count} {partial} | {incompatible_count} {incompatible} | {unknown_count} {unknown}",
        style(format!("Migrateable mods: {}/{}", to_migrate.len() - incompatible_count, to_migrate.len())).bold(),
        compatible = style("compatible").green(),
        partial = style("partial").color256(166),
        incompatible = style("incompatible").red(),
        unknown = style("unknown").blue()
    ).unwrap();

    print!("{}", out);

    if !Confirm::new()
        .with_prompt("Migrate to new version?")
        .interact()
        .unwrap()
    {
        return Ok(());
    }

    if incompatible_count > 0 && Confirm::new()
        .with_prompt("Remove incompatible mods from index?")
        .default(false)
        .interact()
        .unwrap()
    {
        println!("Removing {} incompatible {}", incompatible_count, if incompatible_count == 1 { "addon" } else { "addons" });
        Index::remove_addons(&to_migrate.iter().filter_map(|(addon, _, compat)| match compat {
            Compatibility::Incompatible => Some(addon),
            _ => None
        }).collect::<Vec<&Addon>>()).await?;
    }

    let mut modpack = Arc::into_inner(modpack).unwrap();
    let new_version = Arc::into_inner(new_version).unwrap();
    modpack.versions.loader_version = match modpack.versions.loader_version.as_str() {
        "latest" => "latest".into(),
        _ => get_latest_loader_version(&modpack.versions.loader, &new_version).await?
    };
    modpack.versions.minecraft.clone_from(&new_version);
    Modpack::write(&modpack)?;

    let migrated_addons: Vec<Addon> = to_migrate.into_iter().filter_map(|(addon, version, _)| version.map(|version| Addon {
        source: match addon.source {
            AddonSource::Modrinth(source) => AddonSource::Modrinth(ModrinthSource { version, ..source }),
            AddonSource::Curseforge(source) => AddonSource::Curseforge(CurseforgeSource { version: version.parse::<i32>().unwrap(), ..source }),
            AddonSource::Github(source) => AddonSource::Github(GithubSource { tag: version, ..source }),
        },
        ..addon
    })).collect();

    Index::write_addons(migrated_addons).await?;
    println!("Migrated to {}", new_version);
    Ok(())
}