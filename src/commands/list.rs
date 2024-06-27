use crate::{structs::Index, Result};

pub async fn list(verbose: bool) -> Result<()> {
    let index = Index::read()?;

    let output: Vec<String> = if verbose {
        index.mods.into_iter().map(|m| format!("
{}:
  type: {}
  id: {}
  version: {}
  from: {}
  pinned: {}",
        m.name,
        m.project_type,
        m.id,
        m.version,
        m.platform,
        m.pinned
    )).collect()
    } else {
        index.mods.into_iter().map(|m| format!("{:8} {}", m.id, m.name)).collect()
    };

    println!("{}", output.join("\n"));
    Ok(())
}