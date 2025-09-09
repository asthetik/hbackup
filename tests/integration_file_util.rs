use hbackup::file_util;
use hbackup::job::{CompressFormat, Level};
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::{NamedTempFile, tempdir};

fn get_filename(temp: &Path) -> String {
    temp.file_name().unwrap().to_string_lossy().to_string()
}

#[test]
fn test_copy_dir_nonexistent_src() {
    let src = tempdir().unwrap().path().join("no_such_src");
    let dest = tempdir().unwrap();
    let res = file_util::copy(&src, dest.path());
    assert!(res.is_err());
    let err_msg = format!("{}", res.unwrap_err());
    assert!(err_msg.contains("The path"));
    assert!(err_msg.contains("does not exist"));
}

#[test]
fn test_copy_dir_to_file_error() {
    let src = tempdir().unwrap();
    let dest_file = NamedTempFile::new().unwrap();
    let res = file_util::copy(src.path(), dest_file.path());
    assert!(res.is_err());
    let err_msg = format!("{}", res.unwrap_err());
    assert!(err_msg.contains("Cannot copy directory "));
    assert!(err_msg.contains(" to file "));
}

#[test]
fn test_copy_dir_into_dir_creates_directory() {
    let src = tempdir().unwrap();
    let filename = get_filename(src.path());
    let dest = tempdir().unwrap().path().join(filename);
    // let dest = dest.path().join(filename);
    let res = file_util::copy(src.path(), &dest);
    assert!(res.is_ok());
    assert!(dest.exists());
}

#[test]
fn test_copy_file_into_dir_creates_file() {
    let mut src_file = NamedTempFile::new().unwrap();
    writeln!(src_file, "Hello, World!").unwrap();
    let dest = tempdir().unwrap();
    let res = file_util::copy(src_file.path(), dest.path());
    assert!(res.is_ok());
    let filename = src_file
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let dest = dest.path().join(filename);
    dbg!(&dest);
    assert!(dest.exists());
    let msg = fs::read_to_string(&dest).unwrap().trim_end().to_string();
    assert_eq!(msg, "Hello, World!");
}

#[tokio::test]
async fn test_copy_async_dir_nonexistent_src() {
    let src = tempdir().unwrap().path().join("no_such_src");
    let dest = tempdir().unwrap();
    let res = file_util::copy_async(src.clone(), dest.path().to_path_buf()).await;
    assert!(res.is_err());
    let err_msg = format!("{}", res.unwrap_err());
    assert!(err_msg.contains("The path"));
    assert!(err_msg.contains("does not exist"));
}

#[tokio::test]
async fn test_copy_async_dir_to_file_error() {
    let src = tempdir().unwrap();
    let dest_file = NamedTempFile::new().unwrap();
    let res = file_util::copy_async(src.path().to_path_buf(), dest_file.path().to_path_buf()).await;
    assert!(res.is_err());
    let err_msg = format!("{}", res.unwrap_err());
    assert!(err_msg.contains("Cannot copy directory "));
    assert!(err_msg.contains(" to file "));
}

#[tokio::test]
async fn test_copy_async_dir_into_dir_creates_directory() {
    let src = tempdir().unwrap();
    let filename = get_filename(src.path());
    let dest = tempdir().unwrap().path().join(filename);
    let res = file_util::copy_async(src.path().to_path_buf(), dest.clone()).await;
    assert!(res.is_ok());
    assert!(dest.exists());
}

#[tokio::test]
async fn test_copy_async_file_into_dir_creates_file() {
    let mut src_file = NamedTempFile::new().unwrap();
    writeln!(src_file, "Hello, World!").unwrap();
    let dest = tempdir().unwrap();
    let res = file_util::copy_async(src_file.path().to_path_buf(), dest.path().to_path_buf()).await;
    assert!(res.is_ok());
    let filename = src_file
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let dest = dest.path().join(filename);
    dbg!(&dest);
    assert!(dest.exists());
    let msg = fs::read_to_string(&dest).unwrap().trim_end().to_string();
    assert_eq!(msg, "Hello, World!");
}

#[test]
fn test_compression_nonexistent_source() {
    let dest = tempdir().unwrap();
    let src = tempdir().unwrap().path().join("nonexistent");

    let result = file_util::compression(
        &src,
        dest.path(),
        &CompressFormat::Gzip,
        &Level::Default,
        &None,
    );

    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Source path does not exist"));
}

#[test]
fn test_compression_invalid_destination() {
    let src = NamedTempFile::new().unwrap();
    let mut file = src.as_file();
    writeln!(file, "test content").unwrap();

    let dest_file = NamedTempFile::new().unwrap();

    let result = file_util::compression(
        src.path(),
        dest_file.path(),
        &CompressFormat::Gzip,
        &Level::Default,
        &None,
    );

    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Invalid file type"));
}

#[test]
fn test_compression_gzip_file() {
    let src = NamedTempFile::new().unwrap();
    let mut file = src.as_file();
    writeln!(file, "test content for gzip compression").unwrap();

    let dest = tempdir().unwrap();

    let result = file_util::compression(
        src.path(),
        dest.path(),
        &CompressFormat::Gzip,
        &Level::Default,
        &None,
    );

    assert!(result.is_ok());

    let filename = get_filename(src.path());
    let compressed_file = dest.path().join(format!("{}.gz", filename));
    assert!(compressed_file.exists());
}

#[test]
fn test_compression_gzip_directory() {
    let src = tempdir().unwrap();
    let file1_path = src.path().join("file1.txt");
    let mut file1 = fs::File::create(&file1_path).unwrap();
    writeln!(file1, "content of file 1").unwrap();

    let subdir = src.path().join("subdir");
    fs::create_dir(&subdir).unwrap();
    let file2_path = subdir.join("file2.txt");
    let mut file2 = fs::File::create(&file2_path).unwrap();
    writeln!(file2, "content of file 2").unwrap();

    let dest = tempdir().unwrap();

    let result = file_util::compression(
        src.path(),
        dest.path(),
        &CompressFormat::Gzip,
        &Level::Default,
        &None,
    );

    assert!(result.is_ok());

    let filename = get_filename(src.path());
    let compressed_file = dest.path().join(format!("{}.tar.gz", filename));
    assert!(compressed_file.exists());
}

#[test]
fn test_compression_zip_file() {
    let src = NamedTempFile::new().unwrap();
    let mut file = src.as_file();
    writeln!(file, "test content for zip compression").unwrap();

    let dest = tempdir().unwrap();

    let result = file_util::compression(
        src.path(),
        dest.path(),
        &CompressFormat::Zip,
        &Level::Default,
        &None,
    );

    assert!(result.is_ok());

    let filename = get_filename(src.path());
    let compressed_file = dest.path().join(format!("{}.zip", filename));
    assert!(compressed_file.exists());
}

#[test]
fn test_compression_zip_directory() {
    let src = tempdir().unwrap();
    let file1_path = src.path().join("file1.txt");
    let mut file1 = fs::File::create(&file1_path).unwrap();
    writeln!(file1, "content of file 1").unwrap();

    let subdir = src.path().join("subdir");
    fs::create_dir(&subdir).unwrap();
    let file2_path = subdir.join("file2.txt");
    let mut file2 = fs::File::create(&file2_path).unwrap();
    writeln!(file2, "content of file 2").unwrap();

    let dest = tempdir().unwrap();

    let result = file_util::compression(
        src.path(),
        dest.path(),
        &CompressFormat::Zip,
        &Level::Default,
        &None,
    );

    assert!(result.is_ok());

    let filename = get_filename(src.path());
    let compressed_file = dest.path().join(format!("{}.zip", filename));
    assert!(compressed_file.exists());
}

#[test]
fn test_compression_sevenz_file() {
    let src = NamedTempFile::new().unwrap();
    let mut file = src.as_file();
    writeln!(file, "test content for 7z compression").unwrap();

    let dest = tempdir().unwrap();

    let result = file_util::compression(
        src.path(),
        dest.path(),
        &CompressFormat::Sevenz,
        &Level::Default,
        &None,
    );

    assert!(result.is_ok());

    let filename = get_filename(src.path());
    let compressed_file = dest.path().join(format!("{}.7z", filename));
    assert!(compressed_file.exists());
}

#[test]
fn test_compression_zstd_file() {
    let src = NamedTempFile::new().unwrap();
    let mut file = src.as_file();
    writeln!(file, "test content for zstd compression").unwrap();

    let dest = tempdir().unwrap();

    let result = file_util::compression(
        src.path(),
        dest.path(),
        &CompressFormat::Zstd,
        &Level::Default,
        &None,
    );

    assert!(result.is_ok());

    let filename = get_filename(src.path());
    let compressed_file = dest.path().join(format!("{}.zst", filename));
    assert!(compressed_file.exists());
}

#[test]
fn test_compression_bzip2_file() {
    let src = NamedTempFile::new().unwrap();
    let mut file = src.as_file();
    writeln!(file, "test content for bzip2 compression").unwrap();

    let dest = tempdir().unwrap();

    let result = file_util::compression(
        src.path(),
        dest.path(),
        &CompressFormat::Bzip2,
        &Level::Default,
        &None,
    );

    assert!(result.is_ok());

    let filename = get_filename(src.path());
    let compressed_file = dest.path().join(format!("{}.bz2", filename));
    assert!(compressed_file.exists());
}

#[test]
fn test_compression_xz_file() {
    let src = NamedTempFile::new().unwrap();
    let mut file = src.as_file();
    writeln!(file, "test content for xz compression").unwrap();

    let dest = tempdir().unwrap();

    let result = file_util::compression(
        src.path(),
        dest.path(),
        &CompressFormat::Xz,
        &Level::Default,
        &None,
    );

    assert!(result.is_ok());

    let filename = get_filename(src.path());
    let compressed_file = dest.path().join(format!("{}.xz", filename));
    assert!(compressed_file.exists());
}

#[test]
fn test_compression_lz4_file() {
    let src = NamedTempFile::new().unwrap();
    let mut file = src.as_file();
    writeln!(file, "test content for lz4 compression").unwrap();

    let dest = tempdir().unwrap();

    let result = file_util::compression(
        src.path(),
        dest.path(),
        &CompressFormat::Lz4,
        &Level::Default,
        &None,
    );

    assert!(result.is_ok());

    let filename = get_filename(src.path());
    let compressed_file = dest.path().join(format!("{}.lz4", filename));
    assert!(compressed_file.exists());
}

#[test]
fn test_compression_tar_file() {
    let src = NamedTempFile::new().unwrap();
    let mut file = src.as_file();
    writeln!(file, "test content for tar archiving").unwrap();

    let dest = tempdir().unwrap();

    let result = file_util::compression(
        src.path(),
        dest.path(),
        &CompressFormat::Tar,
        &Level::Default,
        &None,
    );

    assert!(result.is_ok());

    let filename = get_filename(src.path());
    let compressed_file = dest.path().join(format!("{}.tar", filename));
    assert!(compressed_file.exists());
}

#[test]
fn test_compression_tar_directory() {
    let src = tempdir().unwrap();
    let file1_path = src.path().join("file1.txt");
    let mut file1 = fs::File::create(&file1_path).unwrap();
    writeln!(file1, "content of file 1").unwrap();

    let subdir = src.path().join("subdir");
    fs::create_dir(&subdir).unwrap();
    let file2_path = subdir.join("file2.txt");
    let mut file2 = fs::File::create(&file2_path).unwrap();
    writeln!(file2, "content of file 2").unwrap();

    let dest = tempdir().unwrap();

    let result = file_util::compression(
        src.path(),
        dest.path(),
        &CompressFormat::Tar,
        &Level::Default,
        &None,
    );

    assert!(result.is_ok());

    let filename = get_filename(src.path());
    let compressed_file = dest.path().join(format!("{}.tar", filename));
    assert!(compressed_file.exists());
}

#[test]
fn test_compression_with_ignore_list() {
    let src = tempdir().unwrap();

    // Create files
    let file1_path = src.path().join("file1.txt");
    let mut file1 = fs::File::create(&file1_path).unwrap();
    writeln!(file1, "content of file 1").unwrap();

    let ignore_file_path = src.path().join("ignore_me.log");
    let mut ignore_file = fs::File::create(&ignore_file_path).unwrap();
    writeln!(ignore_file, "this should be ignored").unwrap();

    let subdir = src.path().join("subdir");
    fs::create_dir(&subdir).unwrap();
    let file2_path = subdir.join("file2.txt");
    let mut file2 = fs::File::create(&file2_path).unwrap();
    writeln!(file2, "content of file 2").unwrap();

    let ignore_dir = src.path().join("ignore_dir");
    fs::create_dir(&ignore_dir).unwrap();
    let ignored_file_path = ignore_dir.join("ignored.txt");
    let mut ignored_file = fs::File::create(&ignored_file_path).unwrap();
    writeln!(ignored_file, "this should be ignored too").unwrap();

    let dest = tempdir().unwrap();

    let ignore_list = Some(vec!["ignore_me.log".to_string(), "ignore_dir".to_string()]);

    let result = file_util::compression(
        src.path(),
        dest.path(),
        &CompressFormat::Tar,
        &Level::Default,
        &ignore_list,
    );

    assert!(result.is_ok());

    let filename = get_filename(src.path());
    let compressed_file = dest.path().join(format!("{}.tar", filename));
    assert!(compressed_file.exists());
}

#[test]
fn test_compression_all_levels() {
    let src = NamedTempFile::new().unwrap();
    let mut file = src.as_file();
    writeln!(file, "test content for level testing").unwrap();

    let levels = [
        Level::Fastest,
        Level::Faster,
        Level::Default,
        Level::Better,
        Level::Best,
    ];

    for level in &levels {
        let dest = tempdir().unwrap();

        let result =
            file_util::compression(src.path(), dest.path(), &CompressFormat::Gzip, level, &None);

        assert!(result.is_ok());

        let filename = src
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let compressed_file = dest.path().join(format!("{}.gz", filename));
        assert!(compressed_file.exists());
    }
}

#[test]
fn test_compression_creates_destination_directory() {
    let src = NamedTempFile::new().unwrap();
    let mut file = src.as_file();
    writeln!(file, "test content").unwrap();

    let dest_parent = tempdir().unwrap();
    let dest = dest_parent
        .path()
        .join("new")
        .join("destination")
        .join("path");

    // Ensure destination directory doesn't exist yet
    assert!(!dest.exists());

    let result = file_util::compression(
        src.path(),
        &dest,
        &CompressFormat::Gzip,
        &Level::Default,
        &None,
    );

    assert!(result.is_ok());
    assert!(dest.exists());

    let filename = get_filename(src.path());
    let compressed_file = dest.join(format!("{}.gz", filename));
    assert!(compressed_file.exists());
}
