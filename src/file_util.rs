use flate2::{write::GzEncoder, Compression};
use std::io::{BufReader, Read, Write};
use std::{fs, io};
use std::{fs::File, path::Path};
use walkdir::WalkDir;
use zip::{write::FileOptions, ZipWriter};

use crate::{application::CompressFormat, Result};

pub fn compression(src: &Path, dest: &Path, format: &CompressFormat) -> Result<()> {
    assert!(src.exists());
    if !src.is_dir() && !src.is_file() {
        return Err("Does not support compression except for files and directories".into());
    }
    if dest.exists() && !dest.is_dir() {
        return Err("Invalid file type".into());
    }
    fs::create_dir_all(dest)?;

    if src.is_file() {
        match format {
            CompressFormat::Gzip => compress_file_gzip(src, dest),
            CompressFormat::Zip => compress_file_zip(src, dest),
        }
    } else {
        match format {
            CompressFormat::Gzip => compress_dir_gzip(src, dest),
            CompressFormat::Zip => compress_dir_zip(src, dest),
        }
    }
}

fn compress_file_gzip(src: &Path, dest: &Path) -> Result<()> {
    let mut f = src.to_path_buf();
    f.set_extension("gz");
    let file_name = f.file_name().unwrap().to_string_lossy().into_owned();
    let dest = dest.join(&file_name);

    let file = File::create(&dest)?;
    let mut reader = BufReader::new(File::open(src)?);
    let mut encoder = GzEncoder::new(file, Compression::default());
    io::copy(&mut reader, &mut encoder)?;
    encoder.finish()?;

    Ok(())
}

fn compress_file_zip(src: &Path, dest: &Path) -> Result<()> {
    let mut f = src.to_path_buf();
    f.set_extension("zip");
    let file_name = f.file_name().unwrap().to_string_lossy().into_owned();
    let dest = dest.join(&file_name);

    let file = File::create(dest)?;
    let mut zip = ZipWriter::new(file);

    let options = FileOptions::<()>::default();

    let name = src.file_name().unwrap().to_string_lossy();
    zip.start_file(name, options)?;

    let mut src_file = File::open(src)?;
    let mut buffer = Vec::new();
    src_file.read_to_end(&mut buffer)?;

    zip.write_all(&buffer)?;
    zip.finish()?;
    Ok(())
}

fn compress_dir_gzip(src: &Path, dest: &Path) -> Result<()> {
    let file_name = src.file_name().unwrap().to_string_lossy().into_owned();
    let file_name = format!("{}.{}", file_name, "tar.gz");
    let dest = dest.join(&file_name);
    let tar_gz = File::create(dest)?;
    let encoder = GzEncoder::new(tar_gz, Compression::default());
    let mut tar_builder = tar::Builder::new(encoder);

    tar_builder.append_dir_all(&file_name, src)?;
    tar_builder.into_inner()?.finish()?;
    Ok(())
}

fn compress_dir_zip(src: &Path, dest: &Path) -> Result<()> {
    let file_name = src.file_name().unwrap().to_string_lossy().into_owned();
    let file_name = format!("{}.{}", file_name, "zip");
    let dest = dest.join(file_name);
    let file = File::create(dest)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::<()>::default();

    let prefix = src.parent().unwrap_or_else(|| Path::new(""));

    for entry in WalkDir::new(src) {
        let entry = entry?;
        let path = entry.path();
        let name = path.strip_prefix(prefix).unwrap().to_string_lossy().to_string();
        if path.is_dir() {
            zip.add_directory(name, options)?;
        } else {
            zip.start_file(name, options)?;
            let mut f = File::open(path)?;
            io::copy(&mut f, &mut zip)?;
        }
    }
    zip.finish()?;

    Ok(())
}
