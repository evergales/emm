use std::cmp::Ordering;

use crate::{api::curseforge::File, structs::{index::ProjectType, pack::Modpack}};

use super::{get_version_filters, FilterVersions};

impl FilterVersions<File> for Vec<File> {
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

    // should be used on versions that are known to be compatible
    // order: matches game version => more recent => matches primary loader
    fn best_match(mut self, modpack: &Modpack) -> Option<File> {
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
                    match b.file_date.cmp(&a.file_date) {
                        Ordering::Less => Ordering::Less,
                        Ordering::Greater => Ordering::Greater,
                        Ordering::Equal => {
                            // if both were uploaded at the same time
                            // prefer versions that match the primary mod loader of the modpack
                            let a_has_preferred_loader = a.game_versions.contains(&modpack.versions.loader.to_string());
                            let b_has_preferred_loader = b.game_versions.contains(&modpack.versions.loader.to_string());
                            
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