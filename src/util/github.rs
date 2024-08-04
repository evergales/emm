use crate::structs::pack::Modpack;

// try to replace loader & mc version in string
// with {mc_version} & {loader}
pub fn find_filter(string: &str, modpack: &Modpack) -> Option<String> {
    let lowercase_name = string.to_lowercase();

    let parsed = lowercase_name.replace(&modpack.versions.minecraft, "{mc_version}");

    match parsed == lowercase_name {
        false => Some(parsed),
        true => None,
    }
}