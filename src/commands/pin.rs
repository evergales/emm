use console::style;

use crate::{cli::PinArgs, error::Result, structs::index::{Addon, AddonOptions, AddonSource, Index}, MODRINTH};

pub async fn pin(args: PinArgs) -> Result<()> {
    let index = Index::read().await?;

    if let Some(addon) = index.select_addon(&args.addon).cloned() {
        if !addon.options.clone().unwrap_or_default().pinned {
            let addon = Addon { options: Some(AddonOptions { pinned: true, ..addon.options.unwrap_or_default() }), ..addon};
            println!("Pinning {}", addon.name);
            Index::write_addons(vec![addon]).await?;
        } else {
            println!("{}", style(format!("{} is already pinned", addon.name)).color256(166))
        }
    }

    Ok(())
}