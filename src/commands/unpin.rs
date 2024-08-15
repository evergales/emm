use console::style;

use crate::{cli::UnpinArgs, error::Result, structs::index::{Addon, AddonOptions, Index}};

pub async fn unpin(args: UnpinArgs) -> Result<()> {
    let index = Index::read().await?;

    if let Some(addon) = index.select_addon(&args.addon).cloned() {
        if addon.options.clone().unwrap_or_default().pinned {
            let addon = Addon { options: Some(AddonOptions { pinned: false, ..addon.options.unwrap_or_default() }), ..addon};
            println!("Unpinning {}", addon.name);
            Index::write_addons(vec![addon]).await?;
        } else {
            println!("{}", style(format!("{} isnt pinned", addon.name)).color256(166))
        }
    }

    Ok(())
}