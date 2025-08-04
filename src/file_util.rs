//! File compression utilities for hbackup.
//!
//! This module provides functions to compress files and directories
//! using gzip, zip, 7z, zstd, bzip2, and xz formats. It supports both single files and entire directories,
//! and automatically selects the correct compression strategy based on the input type and format.

use crate::application::CompressFormat;
use crate::application::Level;
use anyhow::Result;
use anyhow::anyhow;
use bzip2::Compression as BzCompression;
use bzip2::write::BzEncoder;
use flate2::{Compression, write::GzEncoder};
use lz4::EncoderBuilder as Lz4EncoderBuilder;
use sevenz_rust2::ArchiveWriter;
use sevenz_rust2::encoder_options::LZMA2Options;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use std::{fs, io};
use std::{fs::File, path::Path};
use tar::Builder;
use walkdir::WalkDir;
use xz2::write::XzEncoder;
use zip::{ZipWriter, write::FileOptions};
use zstd::stream::write::Encoder as ZstdEncoder;

/// Compresses a file or directory at `src` into the `dest` directory using the specified `format` and `level`.
///
/// # Arguments
/// * `src` - The source file or directory to compress.
/// * `dest` - The destination directory where the compressed file will be placed.
/// * `format` - The compression format to use (`Gzip`, `Zip`, `Sevenz`, `Zstd`, `Bzip2`, or `Xz`).
/// * `level` - Compression level (see [`Level`]).
///
/// # Errors
/// Returns an error if the source does not exist, is not a file or directory,
/// if the destination is not a directory, or if any IO error occurs during compression.
pub(crate) fn compression(
    src: &Path,
    dest: &Path,
    format: &CompressFormat,
    level: &Level,
    ignore: &Option<Vec<String>>,
) -> Result<()> {
    assert!(src.exists());
    if !src.is_dir() && !src.is_file() {
        return Err(anyhow!(
            "Does not support compression except for files and directories"
        ));
    }
    if dest.exists() && !dest.is_dir() {
        return Err(anyhow!("Invalid file type"));
    }
    fs::create_dir_all(dest)?;

    match format {
        CompressFormat::Gzip => compress_gzip(src, dest, level, ignore),
        CompressFormat::Zip => compress_zip(src, dest, level, ignore),
        CompressFormat::Sevenz => compress_sevenz(src, dest, level, ignore),
        CompressFormat::Zstd => compress_zstd(src, dest, level, ignore),
        CompressFormat::Bzip2 => compress_bzip2(src, dest, level, ignore),
        CompressFormat::Xz => compress_xz(src, dest, level, ignore),
        CompressFormat::Lz4 => compress_lz4(src, dest, level, ignore),
        CompressFormat::Tar => compress_tar(src, dest, ignore),
    }
}

/// Compresses a file or directory at `src` into a gz/tar.gz archive in the `dest` directory.
///
/// # Arguments
/// * `src` - The source directory to compress.
/// * `dest` - The destination directory.
/// * `level` - Compression level.
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_gzip(
    src: &Path,
    dest: &Path,
    level: &Level,
    ignore: &Option<Vec<String>>,
) -> Result<()> {
    let file_name = get_file_name(src);
    let level = match level {
        Level::Fastest => Compression::fast(),
        Level::Faster => Compression::new(3),
        Level::Default => Compression::default(),
        Level::Better => Compression::new(8),
        Level::Best => Compression::best(),
    };

    if src.is_dir() {
        let dest = dest.join(format!("{file_name}.tar.gz"));
        let tar_gz = File::create(dest)?;

        let encoder = GzEncoder::new(tar_gz, level);
        let mut tar_builder = tar::Builder::new(encoder);
        append_regular_only(&mut tar_builder, src, ignore)?;
        tar_builder.into_inner()?.finish()?;
    } else {
        let dest = dest.join(format!("{file_name}.gz"));
        let dest_file = File::create(&dest)?;

        let mut reader = BufReader::new(File::open(src)?);
        let mut encoder = GzEncoder::new(dest_file, level);
        io::copy(&mut reader, &mut encoder)?;
        encoder.finish()?;
    }

    Ok(())
}

/// Compresses a file or directory at `src` into a zip archive in the `dest` directory.
///
/// # Arguments
/// * `src` - The source directory to compress.
/// * `dest` - The destination directory.
/// * `level` - Compression level (1-9).
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_zip(
    src: &Path,
    dest: &Path,
    level: &Level,
    ignore: &Option<Vec<String>>,
) -> Result<()> {
    let file_name = get_file_name(src);
    let dest = dest.join(format!("{file_name}.zip"));
    let dest_file = File::create(dest)?;

    let mut zip = ZipWriter::new(dest_file);
    let level = match level {
        Level::Fastest => 1,
        Level::Faster => 3,
        Level::Default => 6,
        Level::Better => 8,
        Level::Best => 9,
    };
    let options = FileOptions::<()>::default().compression_level(Some(level));
    if src.is_dir() {
        let prefix = src.parent().unwrap_or_else(|| Path::new(""));
        let ignore_path = match ignore {
            Some(ignore) => ignore.iter().map(|s| src.join(s)).collect::<Vec<PathBuf>>(),
            None => vec![],
        };

        for entry in WalkDir::new(src) {
            let entry = entry?;
            let path = entry.path();
            if ignore_path.iter().any(|p| path.starts_with(p)) {
                continue;
            }

            let name = path
                .strip_prefix(prefix)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let md = fs::symlink_metadata(path)?;
            if md.is_dir() {
                zip.add_directory(name, options)?;
            } else if md.is_file() {
                zip.start_file(name, options)?;
                let mut f = File::open(path)?;
                io::copy(&mut f, &mut zip)?;
            }
        }
    } else {
        zip.start_file(file_name, options)?;

        let mut src_file = File::open(src)?;
        let mut buffer = Vec::new();
        src_file.read_to_end(&mut buffer)?;

        zip.write_all(&buffer)?;
        zip.finish()?;
    }

    Ok(())
}

/// Compresses a file or directory at `src` into a 7z archive in the `dest` directory.
///
/// # Arguments
/// * `src` - The source file or directory to compress.
/// * `dest` - The destination directory.
/// * `level` - Compression level (1-9).
///
/// # Errors
/// Returns an error if any IO error occurs or if 7z compression fails.
fn compress_sevenz(
    src: &Path,
    dest: &Path,
    level: &Level,
    ignore: &Option<Vec<String>>,
) -> Result<()> {
    let file_name = get_file_name(src);
    let dest = dest.join(format!("{file_name}.7z"));

    let mut writer = ArchiveWriter::create(dest)?;
    let level = match level {
        Level::Fastest => 1,
        Level::Faster => 3,
        Level::Default => 6,
        Level::Better => 8,
        Level::Best => 9,
    };
    let lzma2 = LZMA2Options::from_level(level).into();
    writer.set_content_methods(vec![lzma2]);
    writer.push_source_path(src, make_filter(src, ignore))?;
    writer.finish()?;

    Ok(())
}

/// Compresses a file or directory at `src` into a zst/tar.zst archive in the `dest` directory.
///
/// # Arguments
/// * `src` - The source directory to compress.
/// * `dest` - The destination directory.
/// * `level` - Compression level (1-22).
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_zstd(
    src: &Path,
    dest: &Path,
    level: &Level,
    ignore: &Option<Vec<String>>,
) -> Result<()> {
    let file_name = get_file_name(src);
    let level = match level {
        Level::Fastest => 1,
        Level::Faster => 2,
        Level::Default => 3,
        Level::Better => 19,
        Level::Best => 22,
    };
    if src.is_dir() {
        let dest = dest.join(format!("{file_name}.tar.zst"));
        let tar_zst = File::create(dest)?;
        let encoder = ZstdEncoder::new(tar_zst, level)?;
        let mut tar_builder = tar::Builder::new(encoder);
        append_regular_only(&mut tar_builder, src, ignore)?;
        tar_builder.into_inner()?.finish()?;
    } else {
        let dest = dest.join(format!("{file_name}.zst"));
        let dest_file = File::create(dest)?;
        let mut reader = BufReader::new(File::open(src)?);
        let mut encoder = ZstdEncoder::new(dest_file, level)?;
        io::copy(&mut reader, &mut encoder)?;
        encoder.finish()?;
    }

    Ok(())
}

/// Compresses a file or directory at `src` into a bz/tar.bz2 archive in the `dest` directory.
///
/// # Arguments
/// * `src` - The source directory to compress.
/// * `dest` - The destination directory
/// * `level` - Compression level.
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_bzip2(
    src: &Path,
    dest: &Path,
    level: &Level,
    ignore: &Option<Vec<String>>,
) -> Result<()> {
    let file_name = get_file_name(src);
    let level = match level {
        Level::Fastest => BzCompression::fast(),
        Level::Faster => BzCompression::new(3),
        Level::Default => BzCompression::default(),
        Level::Better => BzCompression::new(8),
        Level::Best => BzCompression::best(),
    };
    if src.is_dir() {
        let dest = dest.join(format!("{file_name}.tar.bz2"));
        let tar_bz = File::create(dest)?;

        let encoder = BzEncoder::new(tar_bz, level);
        let mut tar_builder = tar::Builder::new(encoder);
        append_regular_only(&mut tar_builder, src, ignore)?;
        tar_builder.into_inner()?.finish()?;
    } else {
        let dest = dest.join(format!("{file_name}.bz2"));
        let dest_file = File::create(dest)?;

        let mut reader = BufReader::new(File::open(src)?);
        let mut encoder = BzEncoder::new(dest_file, level);
        io::copy(&mut reader, &mut encoder)?;
        encoder.finish()?;
    }

    Ok(())
}

/// Compresses a file or directory at `src` into a xz/tar.xz archive in the `dest` directory.
///
/// # Arguments
/// * `src` - The source directory to compress.
/// * `dest` - The destination directory.
/// * `level` - Compression level (1-9).
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_xz(src: &Path, dest: &Path, level: &Level, ignore: &Option<Vec<String>>) -> Result<()> {
    let file_name = get_file_name(src);
    let level = match level {
        Level::Fastest => 1,
        Level::Faster => 3,
        Level::Default => 6,
        Level::Better => 8,
        Level::Best => 9,
    };
    if src.is_dir() {
        let dest = dest.join(format!("{file_name}.tar.xz"));
        let tar_xz = File::create(dest)?;

        let encoder = XzEncoder::new(tar_xz, level);
        let mut tar_builder = tar::Builder::new(encoder);
        append_regular_only(&mut tar_builder, src, ignore)?;
        tar_builder.into_inner()?.finish()?;
    } else {
        let dest = dest.join(format!("{file_name}.xz"));
        let dest_file = File::create(dest)?;

        let mut reader = BufReader::new(File::open(src)?);
        let mut encoder = XzEncoder::new(dest_file, level);
        io::copy(&mut reader, &mut encoder)?;
        encoder.finish()?;
    }

    Ok(())
}

// Compresses a file or directory at `src` into a lz4/tar.lz4 archive in the `dest` directory.
///
/// # Arguments
/// * `src` - The source directory to compress.
/// * `dest` - The destination directory.
/// * `level` - Compression level (1-16).
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_lz4(
    src: &Path,
    dest: &Path,
    level: &Level,
    ignore: &Option<Vec<String>>,
) -> Result<()> {
    let file_name = get_file_name(src);
    let level = match level {
        Level::Fastest => 1,
        Level::Faster => 3,
        Level::Default => 6,
        Level::Better => 14,
        Level::Best => 16,
    };
    if src.is_dir() {
        let dest = dest.join(format!("{file_name}.tar.lz4"));
        let tar_lz = File::create(dest)?;

        let encoder = Lz4EncoderBuilder::new().level(level).build(tar_lz)?;
        let mut tar_builder = tar::Builder::new(encoder);
        append_regular_only(&mut tar_builder, src, ignore)?;
        let (_, result) = tar_builder.into_inner()?.finish();
        result?;
    } else {
        let dest = dest.join(format!("{file_name}.lz4"));
        let dest_file = File::create(dest)?;

        let mut reader = BufReader::new(File::open(src)?);
        let mut encoder = Lz4EncoderBuilder::new().level(level).build(dest_file)?;
        io::copy(&mut reader, &mut encoder)?;
        let (_, result) = encoder.finish();
        result?;
    }

    Ok(())
}

/// Returns the file or directory name as a `String`.
///
/// # Arguments
/// * `file` - The path to extract the file or directory name from.
///
/// # Panics
/// Panics if the path does not have a file name.
fn get_file_name(file: &Path) -> String {
    file.file_name().unwrap().to_string_lossy().into_owned()
}

/// Appends only regular files and directories from `src` into the provided tar archive builder.
///
/// This helper skips symlinks and special files for safety and portability.
///
/// # Arguments
/// * `tar` - The tar archive builder to append files/directories to.
/// * `src` - The source directory to walk and archive.
///
/// # Errors
/// Returns an error if any IO error occurs during traversal or archiving.
fn append_regular_only<W: Write>(
    tar: &mut Builder<W>,
    src: &Path,
    ignore: &Option<Vec<String>>,
) -> Result<()> {
    let prefix = src.parent().unwrap_or(Path::new(""));
    let ignore_paths: Vec<PathBuf> = ignore
        .as_ref()
        .map(|dirs| dirs.iter().map(|s| src.join(s)).collect())
        .unwrap_or_default();

    for entry in WalkDir::new(src) {
        let entry = entry?;
        let path = entry.path();
        if ignore_paths.iter().any(|p| path.starts_with(p)) {
            continue;
        }

        let rel = path.strip_prefix(prefix).unwrap();
        let md = fs::symlink_metadata(path)?;
        if md.is_dir() {
            tar.append_dir(rel, path)?;
        } else if md.is_file() {
            tar.append_path_with_name(path, rel)?;
        }
    }
    Ok(())
}

/// Creates a filter function that determines whether a given path should be ignored based on the provided ignore list.
fn make_filter(base: &Path, ignore: &Option<Vec<String>>) -> impl Fn(&Path) -> bool {
    let ignore_paths: Vec<PathBuf> = ignore
        .as_ref()
        .map(|dirs| dirs.iter().map(|s| base.join(s)).collect())
        .unwrap_or_default();
    move |path| !ignore_paths.iter().any(|p| path.starts_with(p))
}

/// Compresses a file or directory at `src` into a tar archive in the `dest` directory.
///
/// # Arguments
/// * `src` - The source file or directory to archive.
/// * `dest` - The destination directory.
/// * `ignore` - Optional list of files/directories to ignore.
///
/// # Errors
/// Returns an error if any IO error occurs.
fn compress_tar(src: &Path, dest: &Path, ignore: &Option<Vec<String>>) -> Result<()> {
    let file_name = get_file_name(src);

    if src.is_dir() {
        let dest = dest.join(format!("{file_name}.tar"));
        let tar_file = File::create(dest)?;
        let mut tar_builder = tar::Builder::new(tar_file);
        append_regular_only(&mut tar_builder, src, ignore)?;
        tar_builder.into_inner()?;
    } else {
        // For single files, create a tar archive containing just that file
        let dest = dest.join(format!("{file_name}.tar"));
        let tar_file = File::create(dest)?;
        let mut tar_builder = tar::Builder::new(tar_file);
        tar_builder.append_path_with_name(src, file_name)?;
        tar_builder.into_inner()?;
    }

    Ok(())
}
