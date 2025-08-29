//! File compression utilities for hbackup.
//!
//! This module provides functions to compress files and directories
//! using gzip, zip, 7z, zstd, bzip2, and xz formats. It supports both single files and entire directories,
//! and automatically selects the correct compression strategy based on the input type and format.

use crate::Item;
use crate::Strategy;
use crate::application::CompressFormat;
use crate::application::Level;
use anyhow::anyhow;
use anyhow::{Context, Result};
use bzip2::Compression as BzCompression;
use bzip2::write::BzEncoder;
use flate2::{Compression, write::GzEncoder};
use lz4::EncoderBuilder as Lz4EncoderBuilder;
use sevenz_rust2::ArchiveWriter;
use sevenz_rust2::encoder_options::LZMA2Options;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;
use std::{fs, io};
use std::{fs::File, path::Path};
use tar::Builder;
use walkdir::WalkDir;
use xz2::write::XzEncoder;
use zip::{ZipWriter, write::FileOptions};
use zstd::stream::write::Encoder as ZstdEncoder;

const TOLERANCE: Duration = Duration::from_secs(1);

pub(crate) fn execute_item(item: Item) -> Result<()> {
    let Item {
        src,
        dest,
        strategy,
    } = item;

    match strategy {
        Strategy::Copy => {
            if let Some(parent) = dest.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            fs::copy(src, dest)?;
        }
        Strategy::Delete => {
            if dest.exists() {
                fs::remove_file(dest)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) async fn execute_item_async(item: Item) -> Result<()> {
    let Item {
        src,
        dest,
        strategy,
    } = item;

    match strategy {
        Strategy::Copy => {
            if let Some(parent) = dest.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            tokio::fs::copy(src, dest).await?;
        }
        Strategy::Delete if dest.exists() => {
            tokio::fs::remove_file(dest).await?;
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn needs_update(src: &Path, dest: &Path) -> Result<bool> {
    if !dest.exists() {
        return Ok(true);
    }

    let sm = fs::metadata(src).context(format!(
        "Failed to get metadata for source file: {}",
        src.display()
    ))?;
    let dm = fs::metadata(dest).context(format!(
        "Failed to get metadata for destination file: {}",
        dest.display()
    ))?;
    if sm.len() != dm.len() {
        return Ok(true);
    }

    let s_mod = sm.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let d_mod = dm.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    if s_mod > d_mod + TOLERANCE {
        return Ok(true);
    }
    Ok(false)
}

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
    if !src.exists() {
        return Err(anyhow!("Source path does not exist: {}", src.display()));
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;
    use tokio;

    fn create_test_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let file_path = dir.join(name);
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content).unwrap();
        file_path
    }

    fn create_test_directory_structure(base_dir: &Path) -> PathBuf {
        let test_dir = base_dir.join("test_directory");
        fs::create_dir_all(&test_dir).unwrap();

        // Create some files
        create_test_file(
            &test_dir,
            "file1.txt",
            b"Hello, World! This is test file 1.",
        );
        create_test_file(
            &test_dir,
            "file2.log",
            b"Log entry 1\nLog entry 2\nLog entry 3",
        );

        // Create subdirectory
        let sub_dir = test_dir.join("subdir");
        fs::create_dir_all(&sub_dir).unwrap();
        create_test_file(
            &sub_dir,
            "nested.txt",
            b"This is a nested file with some content.",
        );

        test_dir
    }

    #[test]
    fn test_execute_item() -> Result<()> {
        let filename = "hello.txt";
        let content = b"Hello, World!";

        let temp_dir = TempDir::new()?;
        let src = create_test_file(temp_dir.path(), filename, content);
        let dest = temp_dir.path().join("output").join(filename);
        let item = Item::from_copy_strategy(src.clone(), dest.clone());
        dbg!(&item);
        execute_item(item)?;
        assert!(dest.exists());
        assert!(dest.is_file());
        let output = fs::read_to_string(dest)?;
        assert_eq!(output, "Hello, World!");

        let temp_dir = TempDir::new()?;
        let src = create_test_file(temp_dir.path(), filename, content);
        let dest = temp_dir.path().join("output").join(filename);
        let item = Item::from_notupdate_strategy(src.clone(), dest.clone());
        dbg!(&item);
        execute_item(item)?;
        assert!(!dest.exists());

        let temp_dir = TempDir::new()?;
        let src = create_test_file(temp_dir.path(), filename, content);
        let dest = temp_dir.path().join("output").join(filename);
        let item = Item::from_ignore_strategy(src.clone(), dest.clone());
        dbg!(&item);
        execute_item(item)?;
        assert!(!dest.exists());

        let temp_dir = TempDir::new()?;
        let dest = create_test_file(temp_dir.path(), filename, content);
        let item = Item::from_delete_strategy(dest.clone());
        dbg!(&item);
        assert!(dest.exists());
        execute_item(item)?;
        assert!(!dest.exists());

        Ok(())
    }

    #[test]
    fn test_execute_item_async() -> Result<()> {
        let filename = "hello.txt";
        let content = b"Hello, World!";
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        let temp_dir = TempDir::new()?;
        let src = create_test_file(temp_dir.path(), filename, content);
        let dest = temp_dir.path().join("output").join(filename);
        let item = Item::from_copy_strategy(src.clone(), dest.clone());
        dbg!(&item);
        rt.block_on(async {
            let res = execute_item_async(item).await;
            assert!(res.is_ok());
        });
        assert!(dest.exists());
        assert!(dest.is_file());
        let output = fs::read_to_string(dest)?;
        assert_eq!(output, "Hello, World!");

        let temp_dir = TempDir::new()?;
        let src = create_test_file(temp_dir.path(), filename, content);
        let dest = temp_dir.path().join("output").join(filename);
        let item = Item::from_notupdate_strategy(src.clone(), dest.clone());
        dbg!(&item);
        rt.block_on(async {
            let res = execute_item_async(item).await;
            assert!(res.is_ok());
        });
        assert!(!dest.exists());

        let temp_dir = TempDir::new()?;
        let src = create_test_file(temp_dir.path(), filename, content);
        let dest = temp_dir.path().join("output").join(filename);
        let item = Item::from_ignore_strategy(src.clone(), dest.clone());
        dbg!(&item);
        rt.block_on(async {
            let res = execute_item_async(item).await;
            assert!(res.is_ok());
        });
        assert!(!dest.exists());

        let temp_dir = TempDir::new()?;
        let dest = create_test_file(temp_dir.path(), filename, content);
        let item = Item::from_delete_strategy(dest.clone());
        dbg!(&item);
        assert!(dest.exists());
        rt.block_on(async {
            let res = execute_item_async(item).await;
            assert!(res.is_ok());
        });
        assert!(!dest.exists());

        Ok(())
    }

    #[test]
    fn test_get_file_name() {
        let path = Path::new("/home/user/document.txt");
        assert_eq!(get_file_name(path), "document.txt");

        let path = Path::new("simple_file");
        assert_eq!(get_file_name(path), "simple_file");

        let path = Path::new("/path/to/directory/");
        assert_eq!(get_file_name(path), "directory");
    }

    #[test]
    fn test_compression_gzip_file() {
        let temp_dir = TempDir::new().unwrap();
        let source_file =
            create_test_file(temp_dir.path(), "test.txt", b"Hello, Gzip compression!");
        let dest_dir = temp_dir.path().join("output");

        let result = compression(
            &source_file,
            &dest_dir,
            &CompressFormat::Gzip,
            &Level::Default,
            &None,
        );

        assert!(result.is_ok());
        assert!(dest_dir.join("test.txt.gz").exists());
    }

    #[test]
    fn test_compression_gzip_directory() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = create_test_directory_structure(temp_dir.path());
        let dest_dir = temp_dir.path().join("output");

        let result = compression(
            &source_dir,
            &dest_dir,
            &CompressFormat::Gzip,
            &Level::Default,
            &None,
        );

        assert!(result.is_ok());
        assert!(dest_dir.join("test_directory.tar.gz").exists());
    }

    #[test]
    fn test_compression_zip_file() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_test_file(temp_dir.path(), "test.txt", b"Hello, Zip compression!");
        let dest_dir = temp_dir.path().join("output");

        let result = compression(
            &source_file,
            &dest_dir,
            &CompressFormat::Zip,
            &Level::Default,
            &None,
        );

        assert!(result.is_ok());
        assert!(dest_dir.join("test.txt.zip").exists());
    }

    #[test]
    fn test_compression_zip_directory() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = create_test_directory_structure(temp_dir.path());
        let dest_dir = temp_dir.path().join("output");

        let result = compression(
            &source_dir,
            &dest_dir,
            &CompressFormat::Zip,
            &Level::Default,
            &None,
        );

        assert!(result.is_ok());
        assert!(dest_dir.join("test_directory.zip").exists());
    }

    #[test]
    fn test_compression_zstd_file() {
        let temp_dir = TempDir::new().unwrap();
        let source_file =
            create_test_file(temp_dir.path(), "test.txt", b"Hello, Zstd compression!");
        let dest_dir = temp_dir.path().join("output");

        let result = compression(
            &source_file,
            &dest_dir,
            &CompressFormat::Zstd,
            &Level::Default,
            &None,
        );

        assert!(result.is_ok());
        assert!(dest_dir.join("test.txt.zst").exists());
    }

    #[test]
    fn test_compression_zstd_directory() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = create_test_directory_structure(temp_dir.path());
        let dest_dir = temp_dir.path().join("output");

        let result = compression(
            &source_dir,
            &dest_dir,
            &CompressFormat::Zstd,
            &Level::Default,
            &None,
        );

        assert!(result.is_ok());
        assert!(dest_dir.join("test_directory.tar.zst").exists());
    }

    #[test]
    fn test_compression_bzip2_file() {
        let temp_dir = TempDir::new().unwrap();
        let source_file =
            create_test_file(temp_dir.path(), "test.txt", b"Hello, Bzip2 compression!");
        let dest_dir = temp_dir.path().join("output");

        let result = compression(
            &source_file,
            &dest_dir,
            &CompressFormat::Bzip2,
            &Level::Default,
            &None,
        );

        assert!(result.is_ok());
        assert!(dest_dir.join("test.txt.bz2").exists());
    }

    #[test]
    fn test_compression_xz_file() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_test_file(temp_dir.path(), "test.txt", b"Hello, XZ compression!");
        let dest_dir = temp_dir.path().join("output");

        let result = compression(
            &source_file,
            &dest_dir,
            &CompressFormat::Xz,
            &Level::Default,
            &None,
        );

        assert!(result.is_ok());
        assert!(dest_dir.join("test.txt.xz").exists());
    }

    #[test]
    fn test_compression_lz4_file() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_test_file(temp_dir.path(), "test.txt", b"Hello, LZ4 compression!");
        let dest_dir = temp_dir.path().join("output");

        let result = compression(
            &source_file,
            &dest_dir,
            &CompressFormat::Lz4,
            &Level::Default,
            &None,
        );

        assert!(result.is_ok());
        assert!(dest_dir.join("test.txt.lz4").exists());
    }

    #[test]
    fn test_compression_tar_file() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_test_file(temp_dir.path(), "test.txt", b"Hello, TAR archiving!");
        let dest_dir = temp_dir.path().join("output");

        let result = compression(
            &source_file,
            &dest_dir,
            &CompressFormat::Tar,
            &Level::Default,
            &None,
        );

        assert!(result.is_ok());
        assert!(dest_dir.join("test.txt.tar").exists());
    }

    #[test]
    fn test_compression_tar_directory() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = create_test_directory_structure(temp_dir.path());
        let dest_dir = temp_dir.path().join("output");

        let result = compression(
            &source_dir,
            &dest_dir,
            &CompressFormat::Tar,
            &Level::Default,
            &None,
        );

        assert!(result.is_ok());
        assert!(dest_dir.join("test_directory.tar").exists());
    }

    #[test]
    fn test_compression_with_ignore_list() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = create_test_directory_structure(temp_dir.path());

        // Add files that should be ignored
        create_test_file(&source_dir, "ignore_me.log", b"This should be ignored");
        let ignore_dir = source_dir.join("ignore_dir");
        fs::create_dir_all(&ignore_dir).unwrap();
        create_test_file(&ignore_dir, "ignored.txt", b"This file should be ignored");

        let dest_dir = temp_dir.path().join("output");
        let ignore_list = Some(vec!["ignore_me.log".to_string(), "ignore_dir".to_string()]);

        let result = compression(
            &source_dir,
            &dest_dir,
            &CompressFormat::Tar,
            &Level::Default,
            &ignore_list,
        );

        assert!(result.is_ok());
        assert!(dest_dir.join("test_directory.tar").exists());
    }

    #[test]
    fn test_compression_levels() {
        let temp_dir = TempDir::new().unwrap();
        let source_file =
            create_test_file(temp_dir.path(), "test.txt", b"Hello, compression levels!");

        let levels = [
            Level::Fastest,
            Level::Faster,
            Level::Default,
            Level::Better,
            Level::Best,
        ];

        for (i, level) in levels.iter().enumerate() {
            let dest_dir = temp_dir.path().join(format!("output_{}", i));
            let result = compression(&source_file, &dest_dir, &CompressFormat::Gzip, level, &None);

            assert!(result.is_ok());
            assert!(dest_dir.join("test.txt.gz").exists());
        }
    }

    #[test]
    fn test_compression_nonexistent_source() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent.txt");
        let dest_dir = temp_dir.path().join("output");

        let result = compression(
            &nonexistent_path,
            &dest_dir,
            &CompressFormat::Gzip,
            &Level::Default,
            &None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_compression_creates_dest_directory() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_test_file(temp_dir.path(), "test.txt", b"Hello, World!");
        let dest_dir = temp_dir.path().join("new_directory").join("nested");

        // dest_dir doesn't exist yet
        assert!(!dest_dir.exists());

        let result = compression(
            &source_file,
            &dest_dir,
            &CompressFormat::Gzip,
            &Level::Default,
            &None,
        );

        assert!(result.is_ok());
        assert!(dest_dir.exists());
        assert!(dest_dir.join("test.txt.gz").exists());
    }

    #[test]
    fn test_compression_invalid_dest_file() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = create_test_file(temp_dir.path(), "test.txt", b"Hello, World!");
        let dest_file = create_test_file(temp_dir.path(), "dest_file.txt", b"existing file");

        let result = compression(
            &source_file,
            &dest_file,
            &CompressFormat::Gzip,
            &Level::Default,
            &None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_make_filter() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();
        let ignore_list = Some(vec!["debug.log".to_string(), "temp".to_string()]);

        let filter = make_filter(base, &ignore_list);

        // These paths should be filtered out (return false)
        assert!(!filter(&base.join("debug.log")));
        assert!(!filter(&base.join("temp")));
        assert!(!filter(&base.join("temp").join("file.txt")));

        // These paths should not be filtered (return true)
        assert!(filter(&base.join("file.txt")));
        assert!(filter(&base.join("data.json")));
    }

    #[test]
    fn test_make_filter_no_ignore() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();
        let ignore_list: Option<Vec<String>> = None;

        let filter = make_filter(base, &ignore_list);

        // All paths should pass the filter when no ignore list is provided
        assert!(filter(&base.join("any_file.txt")));
        assert!(filter(&base.join("any_directory")));
        assert!(filter(&base.join("nested").join("file.log")));
    }
}
