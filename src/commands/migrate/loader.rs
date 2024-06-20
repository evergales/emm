use dialoguer::Select;

use crate::{structs::{ModLoader, Modpack}, util::get_compatible_loader_versions, Result};

pub async fn migrate_loader() -> Result<()> {
    let mut modpack = Modpack::read()?;
    let mut versions = get_compatible_loader_versions(&modpack.versions.mod_loader, &modpack.versions.minecraft).await?;

    // fabric/quilt maven versions are ordered [oldest..newest] reverse it before showing select menu
    if matches!(modpack.versions.mod_loader, ModLoader::Fabric | ModLoader::Quilt) {
        versions.reverse();
    }

    let selected_idx = Select::new()
        .with_prompt(format!("{} loader version", modpack.versions.mod_loader))
        .items(&versions)
        .interact()?;
    let selected_version = versions[selected_idx].to_owned();

    modpack.versions.loader_version = selected_version;
    Modpack::write(&modpack)?;
    Ok(())
}