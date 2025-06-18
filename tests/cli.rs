use assert_cmd::prelude::*;
use assert_fs::fixture::*;

use predicates::prelude::*;
use std::process::Command;

#[test]
fn file_doesnt_exist() -> Result<(), Box<dyn std::error::Error>> {
    before_test()?;

    let mut cmd = Command::cargo_bin("bk")?;
    cmd.arg("add").arg("-s").arg("foo/bar").arg("-t").arg("bar");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No such file or directory"));

    after_test()?;
    Ok(())
}

#[test]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    before_test()?;

    let file = assert_fs::NamedTempFile::new("sample.txt")?;
    file.write_str("A test\nActual content\nMore content\nAnother test")?;

    let output = Command::cargo_bin("bk")?
        .arg("add")
        .arg("-s")
        .arg("./")
        .arg("-t")
        .arg("bar")
        .output()?;
    assert!(output.status.success());

    let output = Command::cargo_bin("bk")?.arg("list").output()?;
    println!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(output.status.success());

    let output = Command::cargo_bin("bk")?.arg("run").output()?;
    println!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(output.status.success());

    after_test()?;
    Ok(())
}

fn before_test() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::cargo_bin("bk")?
        .arg("config")
        .arg("--reset")
        .output()?;
    println!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(output.status.success());

    Ok(())
}

fn after_test() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::cargo_bin("bk")?
        .arg("config")
        .arg("--rollback")
        .output()?;
    println!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(output.status.success());
    Ok(())
}

