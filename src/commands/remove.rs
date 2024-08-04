use console::style;

use crate::{cli::RemoveArgs, error::Result, structs::index::Index};

pub async fn remove(args: RemoveArgs) -> Result<()> {
    let index = Index::read().await?;
    let mut to_remove = Vec::new();

    for string in args.mods {
        match index.addons.iter().find(|a| a.matches_str(&string)) {
            Some(addon) => {
                println!("Removing {}", addon.name);
                to_remove.push(addon);
            },
            None => println!("{}", style(format!("Couldn't find '{string}' in index")).color256(166)),
        }
    }

    Index::remove_addons(to_remove.as_slice()).await?;
    Ok(())
}