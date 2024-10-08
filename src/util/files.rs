use std::{env, fs::File, io::{Read, Write}, path::Path};

use path_clean::clean;
use walkdir::WalkDir;
use zip::{write::SimpleFileOptions, ZipWriter};

use crate::error::Result;

pub fn is_local_path(path: &Path) -> bool {
    let current_dir = env::current_dir().unwrap();
    path.is_relative() && clean(current_dir.join(path)).starts_with(current_dir)
}

pub async fn download_file(path: &Path, url: &str) -> Result<()> {
    let res = reqwest::get(url).await?;
    let data = &*res.bytes().await?;
    let mut file = File::create(path)?;
    file.write_all(data)?;
    Ok(())
}

// https://github.com/zip-rs/zip2/blob/master/examples/write_dir.rs
pub fn add_recursively(from_path: &Path, zip_path: &Path, zip: &mut ZipWriter<File>, options: SimpleFileOptions) -> zip::result::ZipResult<()> {
    let mut buffer = Vec::new();
    for entry in WalkDir::new(from_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let file_name = path.strip_prefix(from_path).unwrap();
        let path_as_string = file_name.to_str().to_owned().unwrap();

        if path.is_file() {
            zip.start_file_from_path(zip_path.join(path_as_string), options)?;
            let mut f = File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
            buffer.clear()
        }
    }

    Ok(())
}