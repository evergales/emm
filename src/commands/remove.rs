use crate::{structs::Index, util::mod_matches, Result};

pub async fn remove_mod(mods: Vec<String>) -> Result<()> {
    let mut index = Index::read()?;
    for i in mods {
        let selected_mod = index.mods.iter().find(|m| {
            mod_matches(m, &i)
        });

        match selected_mod {
            Some(selected_mod) => {
                let idx = index.mods.iter().position(|m| m == selected_mod).unwrap();
                println!("Removing {}", selected_mod.name);
                index.mods.remove(idx);
            },
            None => {
                println!("{i} was not found in this modpack!");
                continue;
            },
        }
    }

    Index::write(&index)?;
    Ok(())
}