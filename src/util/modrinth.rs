use std::cmp::Ordering;

use crate::{api::modrinth::{SideSupportType, Version, VersionFile}, error::{Error, Result}, structs::{index::{ProjectType, Side}, pack::Modpack}};

use super::{get_version_filters, FilterVersions};

pub fn primary_file(files: Vec<VersionFile>) -> VersionFile {
    let first = files.first().unwrap().clone();
    files.into_iter().find(|f| f.primary).unwrap_or(first)
}

pub fn get_primary_hash(files: Vec<VersionFile>) -> Result<String> {
    let primary = primary_file(files);
    match primary.hashes.get("sha1") {
        Some(hash) => Ok(hash.to_owned()),
        None => Err(Error::Other(format!("{} file does not have a sha1 hash", primary.url))),
    }
}

pub fn get_side(client_side: &SideSupportType, server_side: &SideSupportType) -> Side {
    let use_client = should_use_side(client_side);
    let use_server = should_use_side(server_side);

    match (use_client, use_server) {
        (true, true) => Side::Both,
        (true, false) => Side::Client,
        (false, true) => Side::Server,
        _ => Side::Both, // SideSupportType could be unknown, better to use both as fallback
    }
}

fn should_use_side(support_type: &SideSupportType) -> bool {
    matches!(support_type, SideSupportType::Required | SideSupportType::Optional)
}

impl FilterVersions<Version> for Vec<Version> {
    fn filter_compatible(self, modpack: &Modpack, project_type: &ProjectType) -> Self {
        let (acceptable_versions, acceptable_loaders) = get_version_filters(modpack);

        self.into_iter().filter(|version|
            acceptable_versions.iter().any(|av|  version.game_versions.contains(av))
            && if matches!(project_type, ProjectType::Mod) {
                acceptable_loaders.iter().any(|al| version.loaders.contains(&al.to_string().to_lowercase()))
            } else { true }
        ).collect()
    }

    // should be used on versions that are known to be compatible
    // order: matches game version => more recent => matches primary loader
    fn best_match(mut self, modpack: &Modpack) -> Option<Version> {
        self.sort_by(|a, b| {
            // prefer versions that match the primary game version of the modpack
            let a_has_preferred_version = a.game_versions.contains(&modpack.versions.minecraft);
            let b_has_preferred_version = b.game_versions.contains(&modpack.versions.minecraft);

            match (a_has_preferred_version, b_has_preferred_version) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => {
                    // if both match the primary game version
                    // prefer more recent versions
                    match b.date_published.cmp(&a.date_published) {
                        Ordering::Less => Ordering::Less,
                        Ordering::Greater => Ordering::Greater,
                        Ordering::Equal => {
                            // if both were uploaded at the same time
                            // prefer versions that match the primary mod loader of the modpack
                            let a_has_preferred_loader = a.loaders.contains(&modpack.versions.loader.to_string().to_lowercase());
                            let b_has_preferred_loader = b.loaders.contains(&modpack.versions.loader.to_string().to_lowercase());
                            
                            match (a_has_preferred_loader, b_has_preferred_loader) {
                                (true, false) => Ordering::Less,
                                (false, true) => Ordering::Greater,
                                _ => Ordering::Equal
                            }
                        },
                    }
                }
            }
        });

        self.first().cloned()
    }
}