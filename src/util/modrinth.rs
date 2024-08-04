use crate::{api::modrinth::VersionFile, error::{Error, Result}};

pub fn primary_file(files: Vec<VersionFile>) -> VersionFile {
    files.into_iter().find(|f| f.primary).unwrap()
}

pub fn get_primary_hash(files: Vec<VersionFile>) -> Result<String> {
    let primary = primary_file(files);
    match primary.hashes.get("sha1") {
        Some(hash) => Ok(hash.to_owned()),
        None => Err(Error::Other(format!("{} file does not have a sha1 hash", primary.url))),
    }
}