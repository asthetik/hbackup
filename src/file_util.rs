//! File compression utilities for hbackup.
//!
//! This module provides functions to compress files and directories
//! using gzip, zip, 7z, zstd, bzip2, and xz formats. It supports both single files and entire directories,
//! and automatically selects the correct compression strategy based on the input type and format.

use crate::{Result, application::CompressFormat};
use bzip2::Compression as BzCompression;
use bzip2::write::BzEncoder;
use flate2::{Compression, write::GzEncoder};
use std::io::{BufReader, Read, Write};
use std::{fs, io};
use std::{fs::File, path::Path};
use walkdir::WalkDir;
use xz2::write::XzEncoder;
use zip::{ZipWriter, write::FileOptions};
use zstd::stream::write::Encoder as ZstdEncoder;

/// Compresses a file or directory at `src` into the `dest` directory using the specified `format`.
///
/// # Arguments
/// * `src` - The source file or directory to compress.
/// * `dest` - The destination directory where the compressed file will be placed.
/// * `format` - The compression format to use (`Gzip`, `Zip`, `Sevenz`, `Zstd`, `Bzip2`, or `Xz`).
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
            CompressFormat::Xz => compress_dir_xz(src, dest),
        }
    } else {
        match format {
            CompressFormat::Gzip => compress_file_gzip(src, dest),
            CompressFormat::Zip => compress_file_zip(src, dest),
            CompressFormat::Sevenz => compress_sevenz(src, dest),
            CompressFormat::Zstd => compress_file_zstd(src, dest),
            CompressFormat::Bzip2 => compress_file_bzip2(src, dest),
            CompressFormat::Xz => compress_file_xz(src, dest),
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
    let file_name = get_file_name(src);
    let dest = dest.join(format!("{file_name}.gz"));
    let dest_file = File::create(&dest)?;

    let mut reader = BufReader::new(File::open(src)?);
    let mut encoder = GzEncoder::new(dest_file, Compression::default());
    io::copy(&mut reader, &mut encoder)?;
    encoder.finish()?;

    Ok(())
}

/// Compresses a directory at `src` into a tar.gz archive in the `dest` directory.
///
/// The output file will have a `.tar.gz` extension and will contain all files and subdirectories.
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_dir_gzip(src: &Path, dest: &Path) -> Result<()> {
    let file_name = get_file_name(src);
    let dest = dest.join(format!("{file_name}.tar.gz"));
    let tar_gz = File::create(dest)?;

    let encoder = GzEncoder::new(tar_gz, Compression::default());
    let mut tar_builder = tar::Builder::new(encoder);
    tar_builder.append_dir_all(&file_name, src)?;
    tar_builder.into_inner()?.finish()?;
    Ok(())
}

/// Compresses a single file at `src` into a zip file in the `dest` directory.
///
/// The output file will have a `.zip` extension.
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_file_zip(src: &Path, dest: &Path) -> Result<()> {
    let file_name = get_file_name(src);
    let dest = dest.join(format!("{file_name}.zip"));
    let dest_file = File::create(dest)?;

    let mut zip = ZipWriter::new(dest_file);
    let options = FileOptions::<()>::default();
    zip.start_file(file_name, options)?;

    let mut src_file = File::open(src)?;
    let mut buffer = Vec::new();
    src_file.read_to_end(&mut buffer)?;

    zip.write_all(&buffer)?;
    zip.finish()?;
    Ok(())
}

/// Compresses a directory at `src` into a zip archive in the `dest` directory.
///
/// The output file will have a `.zip` extension and will contain all files and subdirectories.
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_dir_zip(src: &Path, dest: &Path) -> Result<()> {
    let file_name = get_file_name(src);
    let dest = dest.join(format!("{file_name}.zip"));
    let dest_file = File::create(dest)?;

    let mut zip = ZipWriter::new(dest_file);
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

/// Compresses a file or directory at `src` into a 7z archive in the `dest` directory.
///
/// The output file will have a `.7z` extension.
///
/// # Errors
/// Returns an error if any IO error occurs or if 7z compression fails.
fn compress_sevenz(src: &Path, dest: &Path) -> Result<()> {
    let file_name = get_file_name(src);
    let dest = dest.join(format!("{file_name}.7z"));
    sevenz_rust2::compress_to_path(src, &dest)?;
    Ok(())
}

/// Compresses a single file at `src` into a zstd file in the `dest` directory.
///
/// The output file will have a `.zst` extension.
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_file_zstd(src: &Path, dest: &Path) -> Result<()> {
    let file_name = get_file_name(src);
    let dest = dest.join(format!("{file_name}.zst"));
    let dest_file = File::create(dest)?;

    let mut reader = BufReader::new(File::open(src)?);
    let mut encoder = ZstdEncoder::new(dest_file, 0)?;
    io::copy(&mut reader, &mut encoder)?;
    encoder.finish()?;
    Ok(())
}

/// Compresses a directory at `src` into a tar.zst archive in the `dest` directory.
///
/// The output file will have a `.tar.zst` extension and will contain all files and subdirectories.
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_dir_zstd(src: &Path, dest: &Path) -> Result<()> {
    let file_name = get_file_name(src);
    let dest = dest.join(format!("{file_name}.tar.zst"));
    let tar_zst = File::create(dest)?;

    let encoder = ZstdEncoder::new(tar_zst, 0)?;
    let mut tar_builder = tar::Builder::new(encoder);
    tar_builder.append_dir_all(&file_name, src)?;
    tar_builder.into_inner()?.finish()?;
    Ok(())
}

/// Compresses a single file at `src` into a bzip2 file in the `dest` directory.
///
/// The output file will have a `.bz2` extension.
///
/// # Errors
/// Returns an error if any IO error occurs.
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

/// Compresses a directory at `src` into a tar.bz2 archive in the `dest` directory.
///
/// The output file will have a `.tar.bz2` extension and will contain all files and subdirectories.
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_dir_bzip2(src: &Path, dest: &Path) -> Result<()> {
    let file_name = get_file_name(src);
    let dest = dest.join(format!("{file_name}.tar.bz2"));
    let tar_bz = File::create(dest)?;

    let encoder = BzEncoder::new(tar_bz, BzCompression::default());
    let mut tar_builder = tar::Builder::new(encoder);
    tar_builder.append_dir_all(&file_name, src)?;
    tar_builder.into_inner()?.finish()?;
    Ok(())
}

/// Compresses a single file at `src` into an xz file in the `dest` directory.
///
/// The output file will have a `.xz` extension.
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_file_xz(src: &Path, dest: &Path) -> Result<()> {
    let file_name = get_file_name(src);
    let dest = dest.join(format!("{file_name}.xz"));
    let dest_file = File::create(dest)?;

    let mut reader = BufReader::new(File::open(src)?);
    let mut encoder = XzEncoder::new(dest_file, 6);
    io::copy(&mut reader, &mut encoder)?;
    encoder.finish()?;
    Ok(())
}

/// Compresses a directory at `src` into a tar.xz archive in the `dest` directory.
///
/// The output file will have a `.tar.xz` extension and will contain all files and subdirectories.
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_dir_xz(src: &Path, dest: &Path) -> Result<()> {
    let file_name = get_file_name(src);
    let dest = dest.join(format!("{file_name}.tar.xz"));
    let tar_xz = File::create(dest)?;

    let encoder = XzEncoder::new(tar_xz, 6);
    let mut tar_builder = tar::Builder::new(encoder);
    tar_builder.append_dir_all(&file_name, src)?;
    tar_builder.into_inner()?.finish()?;
    Ok(())
}

/// Returns the file or directory name as a `String`.
///
/// # Panics
/// Panics if the path does not have a file name.
fn get_file_name(file: &Path) -> String {
    file.file_name().unwrap().to_string_lossy().into_owned()
}
