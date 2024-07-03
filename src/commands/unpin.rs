use console::style;

use crate::{structs::Index, util::mod_matches, Result};

pub async fn unpin(mod_str: String) -> Result<()> {
    let mut index = Index::read()?;
    let index_mod = index.mods.iter().find(|m| {
        mod_matches(m, &mod_str)
    });

    match index_mod {
        Some(index_mod) => {
            let idx = index.mods.iter().position(|m| m == index_mod).unwrap();
            println!("Unpinning {}!", index_mod.name);
            index.mods[idx].pinned = false;
        },
        None => {
            println!("{}", style("Could not find".to_owned() + &mod_str + "in this modpack!").color256(166))
        },
    }

    Index::write(&index)?;
    Ok(())
}