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

impl FilterVersions for Vec<Version> {
    fn filter_compatible(self, modpack: &Modpack, project_type: &ProjectType) -> Self {
        let (acceptable_versions, acceptable_loaders) = get_version_filters(modpack);

        self.into_iter().filter(|version|
            acceptable_versions.iter().any(|av|  version.game_versions.contains(av))
            && if matches!(project_type, ProjectType::Mod) {
                acceptable_loaders.iter().any(|al| version.loaders.contains(&al.to_string().to_lowercase()))
            } else { true }
        ).collect()
    }
}