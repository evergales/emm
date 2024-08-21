use console::style;

use crate::{cli::RemoveArgs, error::Result, structs::index::Index};

pub async fn remove(args: RemoveArgs) -> Result<()> {
    let index = Index::read().await?;
    let mut to_remove = Vec::new();

    for string in args.addons {
        match index.select_addon(&string) {
            Some(addon) => {
                println!("Removing {}", addon.name);
                to_remove.push(addon);
            },
            None => println!("{}", style(format!("Skipping '{}'", string)).dim()),
        }
    }

    Index::remove_addons(to_remove.as_slice()).await?;
    Ok(())
}