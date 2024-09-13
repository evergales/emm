use crate::{api::curseforge::File, structs::{index::ProjectType, pack::Modpack}};

use super::{get_version_filters, FilterVersions};

impl FilterVersions for Vec<File> {
    fn filter_compatible(self, modpack: &Modpack, project_type: &ProjectType) -> Self {
        let (acceptable_versions, acceptable_loaders) = get_version_filters(modpack);

        self.into_iter().filter(|file|
            file.is_available
            && acceptable_versions.iter().any(|av| file.game_versions.contains(av))
            && if matches!(project_type, ProjectType::Mod) {
                acceptable_loaders.iter().any(|al| file.game_versions.contains(&al.to_string()))
            } else { true }
        ).collect()
    }
}