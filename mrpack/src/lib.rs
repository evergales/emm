use std::{fs::File, io::Write, path::PathBuf};

use structs::Metadata;
use zip::{write::SimpleFileOptions, ZipWriter};

pub mod structs;

pub fn create(path: PathBuf, metadata: Metadata, overrides: Option<PathBuf>, mod_overrides: Option<PathBuf>) -> zip::result::ZipResult<()> {
    let zip_path: PathBuf = path.join(format!("{}-{}.mrpack", metadata.name, metadata.version_id));
    let mut zip = ZipWriter::new(File::create(zip_path).unwrap());
    let compression = zip::CompressionMethod::Deflated;
    zip.start_file(
        "modrinth.index.json",
        SimpleFileOptions::default().compression_method(compression),
    )?;
    let metadata_str = serde_json::to_string_pretty(&metadata).unwrap();
    zip.write_all(metadata_str.as_bytes())?;

    if overrides.is_some() {
        zip.add_directory_from_path(
            overrides.unwrap(),
            SimpleFileOptions::default().compression_method(compression),
        )?
    }

    zip.finish()?;
    Ok(())
}
