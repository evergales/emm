use crate::{structs::Index, Result};
use super::mod_matches;

pub async fn pin(mod_str: String) -> Result<()> {
    let mut index = Index::read()?;
    let index_mod = index.mods.iter().find(|m| {
        mod_matches(m, &mod_str)
    });

    match index_mod {
        Some(index_mod) => {
            let idx = index.mods.iter().position(|m| m == index_mod).unwrap();
            println!("Pinning {}!", index_mod.name);
            index.mods[idx].pinned = Some(true);
        },
        None => {
            println!("Could not find {mod_str} in this modpack!")
        },
    }

    Index::write(&index)?;
    Ok(())
}