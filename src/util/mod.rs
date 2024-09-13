use supports_hyperlinks::supports_hyperlinks;

use crate::structs::{index::ProjectType, pack::{ModLoader, Modpack}};

pub mod pack;
pub mod index;
pub mod versions;
pub mod modrinth;
pub mod curseforge;
pub mod files;

// using https://crates.io/crates/supports-hyperlinks
// to test if hyperlinks in terminal are supported and use a link if they are
pub fn to_hyperlink(link: &str, placeholder: &str) -> String {
    if supports_hyperlinks() {
        format!("\u{1b}]8;;{}\u{1b}\\{}\u{1b}]8;;\u{1b}\\", link, placeholder)
    } else {
        placeholder.into()
    }
}

pub trait FilterVersions {
    fn filter_compatible(self, modpack: &Modpack, project_type: &ProjectType) -> Self;
}

pub fn get_version_filters(modpack: &Modpack) -> (Vec<&String>, Vec<&ModLoader>) {
    let mut acceptable_versions = vec![&modpack.versions.minecraft];
    if let Some(versions) = modpack.options.acceptable_versions.as_ref() {
        acceptable_versions.extend(versions);
    }

    let mut acceptable_loaders = vec![&modpack.versions.loader];
    if let Some(loaders) = modpack.options.acceptable_loaders.as_ref() {
        acceptable_loaders.extend(loaders);
    }

    (acceptable_versions, acceptable_loaders)
}