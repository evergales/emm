use std::{fmt::Write, sync::{Arc, Mutex}};

use console::style;

use crate::{cli::{AddArgs, AddCommands}, error::Result, structs::index::{Addon, Index}};

pub mod curseforge;
pub mod github;
pub mod modrinth;

pub async fn add(args: AddArgs) -> Result<()> {
    match args.subcommand {
        AddCommands::Modrinth(args) => modrinth::add_modrinth(args).await,
        AddCommands::Curseforge(args) => curseforge::add_curseforge(args).await,
        AddCommands::Github(args) => github::add_github(args).await
    }
}

pub async fn add_to_index(addons: Vec<Addon>) -> Result<()> {
    let index = Index::read().await?;
    let mut to_add = Vec::new();

    let mut out = format!("Adding{}", if addons.len() > 1 {":\n"} else {" "});

    for addon in addons {
        // checking the name as well so you cant add the same mod from both modrinth or curseforge
        if index.addons.iter().any(|idx_mod| idx_mod.name == addon.name || idx_mod.generic_id() == addon.generic_id()) {
            writeln!(&mut out, "{} {}", &addon.name, style("(already in the modpack)").dim()).unwrap();
            continue;
        }

        writeln!(&mut out, "{}", &addon.name).unwrap();
        to_add.push(addon)
    }

    print!("{}", out);
    Index::write_addons(to_add).await?;
    Ok(())
}

// using another non-async function to lock the mutex to not have a "held across await" error
fn handle_checked(id: &String, checked_ids: &Arc<Mutex<Vec<String>>>) -> bool {
    let mut checked_lock = checked_ids.lock().unwrap();
    let is_checked = checked_lock.contains(id);
    if !is_checked {
        checked_lock.push(id.to_owned());
    }

    is_checked
}