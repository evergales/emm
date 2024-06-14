use std::{fs::File, io::{Read, Write}, path::PathBuf};

use structs::Metadata;
use walkdir::WalkDir;
use zip::{write::SimpleFileOptions, ZipWriter};

pub mod structs;

pub fn create(path: PathBuf, metadata: Metadata, overrides: Option<PathBuf>, mod_overrides: Option<PathBuf>) -> zip::result::ZipResult<()> {
    let zip_path: PathBuf = path.join(format!("{}-{}.mrpack", metadata.name, metadata.version_id));
    let mut zip = ZipWriter::new(File::create(zip_path).unwrap());
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(
        "modrinth.index.json",
        options,
    )?;
    let metadata_str = serde_json::to_string_pretty(&metadata).unwrap();
    zip.write_all(metadata_str.as_bytes())?;

    if overrides.is_some() {
        zip.add_directory("overrides", options)?;
        add_recursively(overrides.unwrap(), "overrides".into(), &mut zip, options)?;
    }

    if mod_overrides.is_some() {
        zip.add_directory("overrides/mods", options)?;
        add_recursively(mod_overrides.unwrap(), "overrides/mods".into(), &mut zip, options)?;
    }

    zip.finish()?;
    Ok(())
}

// https://github.com/zip-rs/zip2/blob/master/examples/write_dir.rs
fn add_recursively(from_path: PathBuf, zip_path: PathBuf, zip: &mut ZipWriter<File>, options: SimpleFileOptions) -> zip::result::ZipResult<()> {
    let mut buffer = Vec::new();
    for entry in WalkDir::new(&from_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let file_name = path.strip_prefix(&from_path).unwrap();
        let path_as_string = file_name.to_str().to_owned().unwrap();

        if path.is_file() {
            zip.start_file_from_path(zip_path.join(path_as_string), options)?;
            let mut f = File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
            buffer.clear()
        } else if !file_name.as_os_str().is_empty() {
            zip.add_directory(path_as_string, options)?;
        }
    }

    Ok(())
}
