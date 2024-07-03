use console::style;

use crate::{structs::Index, Result};

pub async fn list(verbose: bool) -> Result<()> {
    let index = Index::read()?;

    let output: Vec<String> = if verbose {
        index.mods.into_iter().map(|m| format!("
{}:
  type: {}
  id: {}
  version: {}
  from: {}{pinned}",
        style(m.name).bold(),
        m.project_type,
        m.id,
        m.version,
        m.platform,
        pinned = if m.pinned {"\n  pinned: true"} else {""}
    )).collect()
    } else {
        // mr ids are 8c long, cf ids are 6c long, use the max length to make the output look a bit nicer
        let id_width = index.mods.iter().max_by_key(|m| m.id.len()).unwrap().id.len();
        index.mods.into_iter().map(|m| {format!("{:id_width$} {}", style(m.id).dim(), m.name)}).collect()
    };

    println!("{}", output.join("\n"));
    Ok(())
}