use hbackup::file_util;
use std::fs;
use tempfile::{NamedTempFile, tempdir};

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
    let filename = src
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let dest = tempdir().unwrap().path().join(filename);
    // let dest = dest.path().join(filename);
    let res = file_util::copy(src.path(), &dest);
    assert!(res.is_ok());
    assert!(dest.exists());
}

#[test]
fn test_copy_file_into_dir_creates_file() {
    use std::io::Write;

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
    let filename = src
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let dest = tempdir().unwrap().path().join(filename);
    let res = file_util::copy_async(src.path().to_path_buf(), dest.clone()).await;
    assert!(res.is_ok());
    assert!(dest.exists());
}

#[tokio::test]
async fn test_copy_async_file_into_dir_creates_file() {
    use std::io::Write;

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
