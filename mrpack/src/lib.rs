use std::{io::Write, path::PathBuf};

use structs::Metadata;
use zip::{write::SimpleFileOptions, ZipWriter};

pub mod structs;

pub fn create(path: PathBuf, metadata: Metadata, overrides: Option<PathBuf>) -> zip::result::ZipResult<()> {
    let mut zip = ZipWriter::new(std::fs::File::create(path).unwrap());
    let compression = zip::CompressionMethod::Deflated;
    zip.start_file(
        "modrinth.index.json",
        SimpleFileOptions::default().compression_method(compression),
    )?;
    let metadata_str = serde_json::to_string(&metadata).unwrap();
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
