//! File compression utilities for hbackup.
//!
//! This module provides functions to compress files and directories
//! using gzip or zip formats, supporting both single files and entire directories.

use crate::{application::CompressFormat, Result};
use bzip2::write::BzEncoder;
use bzip2::Compression as BzCompression;
use flate2::{write::GzEncoder, Compression};
use std::io::{BufReader, Read, Write};
use std::{fs, io};
use std::{fs::File, path::Path};
use walkdir::WalkDir;
use zip::{write::FileOptions, ZipWriter};
use zstd::stream::write::Encoder as ZstdEncoder;

/// Compresses a file or directory at `src` into the `dest` directory using the specified `format`.
///
/// # Arguments
/// * `src` - The source file or directory to compress.
/// * `dest` - The destination directory where the compressed file will be placed.
/// * `format` - The compression format to use (`Gzip` or `Zip`).
///
/// # Errors
/// Returns an error if the source does not exist, is not a file or directory,
/// if the destination is not a directory, or if any IO error occurs during compression.
pub fn compression(src: &Path, dest: &Path, format: &CompressFormat) -> Result<()> {
    assert!(src.exists());
    if !src.is_dir() && !src.is_file() {
        return Err("Does not support compression except for files and directories".into());
    }
    if dest.exists() && !dest.is_dir() {
        return Err("Invalid file type".into());
    }
    fs::create_dir_all(dest)?;

    if src.is_dir() {
        match format {
            CompressFormat::Gzip => compress_dir_gzip(src, dest),
            CompressFormat::Zip => compress_dir_zip(src, dest),
            CompressFormat::Sevenz => compress_sevenz(src, dest),
            CompressFormat::Zstd => compress_dir_zstd(src, dest),
            CompressFormat::Bzip2 => compress_dir_bzip2(src, dest),
        }
    } else {
        match format {
            CompressFormat::Gzip => compress_file_gzip(src, dest),
            CompressFormat::Zip => compress_file_zip(src, dest),
            CompressFormat::Sevenz => compress_sevenz(src, dest),
            CompressFormat::Zstd => compress_file_zstd(src, dest),
            CompressFormat::Bzip2 => compress_file_bzip2(src, dest),
        }
    }
}

/// Compresses a single file at `src` into a gzip file in the `dest` directory.
///
/// The output file will have a `.gz` extension.
///
/// # Errors
/// Returns an error if any IO error occurs.
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

/// Compresses a single file at `src` into a zip file in the `dest` directory.
///
/// The output file will have a `.zip` extension.
///
/// # Errors
/// Returns an error if any IO error occurs.
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

/// Compresses a directory at `src` into a tar.gz archive in the `dest` directory.
///
/// The output file will have a `.tar.gz` extension and will contain all files and subdirectories.
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_dir_gzip(src: &Path, dest: &Path) -> Result<()> {
    let file_name = src.file_name().unwrap().to_string_lossy().into_owned();
    let file_name = format!("{file_name}.tar.gz");
    let dest = dest.join(&file_name);
    let tar_gz = File::create(dest)?;
    let encoder = GzEncoder::new(tar_gz, Compression::default());
    let mut tar_builder = tar::Builder::new(encoder);

    tar_builder.append_dir_all(&file_name, src)?;
    tar_builder.into_inner()?.finish()?;
    Ok(())
}

/// Compresses a directory at `src` into a zip archive in the `dest` directory.
///
/// The output file will have a `.zip` extension and will contain all files and subdirectories.
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_dir_zip(src: &Path, dest: &Path) -> Result<()> {
    let file_name = src.file_name().unwrap().to_string_lossy().into_owned();
    let file_name = format!("{file_name}.zip");
    let dest = dest.join(file_name);
    let file = File::create(dest)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::<()>::default();

    let prefix = src.parent().unwrap_or_else(|| Path::new(""));

    for entry in WalkDir::new(src) {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .strip_prefix(prefix)
            .unwrap()
            .to_string_lossy()
            .to_string();
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

fn compress_sevenz(src: &Path, dest: &Path) -> Result<()> {
    let file = if src.is_dir() {
        src.to_path_buf()
    } else {
        let mut file = src.to_path_buf();
        file.set_extension("");
        file
    };
    let name = file.file_name().unwrap().to_string_lossy().into_owned();
    let name = format!("{name}.7z");
    let dest = dest.join(name);
    sevenz_rust2::compress_to_path(src, &dest)?;
    Ok(())
}

fn compress_file_zstd(src: &Path, dest: &Path) -> Result<()> {
    let mut file = src.to_path_buf();
    file.set_extension("zst");
    let name = get_file_name(&file);
    let dest = dest.join(name);
    let dest_file = File::create(dest)?;

    let mut reader = BufReader::new(File::open(src)?);
    let mut encoder = ZstdEncoder::new(dest_file, 0)?;
    io::copy(&mut reader, &mut encoder)?;
    encoder.finish()?;
    Ok(())
}

fn compress_dir_zstd(src: &Path, dest: &Path) -> Result<()> {
    let name = get_file_name(src);
    let name = format!("{name}.tar.zst");
    let dest = dest.join(&name);
    let tar_szt = File::create(dest)?;

    let encoder = ZstdEncoder::new(tar_szt, 0)?;
    let mut tar_builder = tar::Builder::new(encoder);
    tar_builder.append_dir_all(&name, src)?;
    tar_builder.into_inner()?.finish()?;
    Ok(())
}

fn compress_file_bzip2(src: &Path, dest: &Path) -> Result<()> {
    let file_name = get_file_name(src);
    let dest = dest.join(format!("{file_name}.bz2"));
    let dest_file = File::create(dest)?;

    let mut reader = BufReader::new(File::open(src)?);
    let mut encoder = BzEncoder::new(dest_file, BzCompression::default());
    io::copy(&mut reader, &mut encoder)?;
    encoder.finish()?;
    Ok(())
}

fn compress_dir_bzip2(src: &Path, dest: &Path) -> Result<()> {
    let dir_name = get_file_name(src);
    let dest = dest.join(format!("{dir_name}.tar.bz2"));
    let tar_bz = File::create(dest)?;

    let encoder = BzEncoder::new(tar_bz, BzCompression::default());
    let mut tar_builder = tar::Builder::new(encoder);
    tar_builder.append_dir_all(&dir_name, src)?;
    tar_builder.into_inner()?.finish()?;
    Ok(())
}

fn get_file_name(file: &Path) -> String {
    file.file_name().unwrap().to_string_lossy().into_owned()
}
