use assert_cmd::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn delete_yes_end_to_end() -> anyhow::Result<()> {
    // Set up temporary config dir
    let temp = TempDir::new()?;

    // Create real source files so `bk add` succeeds
    let bin = assert_cmd::cargo::cargo_bin!("bk");
    let src1 = temp.path().join("src1.txt");
    std::fs::write(&src1, b"hello")?;
    let tgt1 = temp.path().join("tgt1");
    std::fs::create_dir_all(&tgt1)?;

    let mut add1 = Command::new(&bin);
    add1.env("XDG_CONFIG_HOME", temp.path())
        .arg("add")
        .arg(src1.as_os_str())
        .arg(tgt1.as_os_str());
    add1.assert().success();

    let src2 = temp.path().join("src2.txt");
    std::fs::write(&src2, b"world")?;
    let tgt2 = temp.path().join("tgt2");
    std::fs::create_dir_all(&tgt2)?;

    let mut add2 = Command::new(&bin);
    add2.env("XDG_CONFIG_HOME", temp.path())
        .arg("add")
        .arg(src2.as_os_str())
        .arg(tgt2.as_os_str());
    add2.assert().success();

    // Verify config contains jobs by running `bk list` and checking output
    let mut list = Command::new(&bin);
    list.env("XDG_CONFIG_HOME", temp.path()).arg("list");
    list.assert()
        .success()
        .stdout(predicate::str::contains("id:"));

    // Run the bk binary with delete --all -y
    let mut cmd = Command::new(&bin);
    cmd.env("XDG_CONFIG_HOME", temp.path())
        .arg("delete")
        .arg("--all")
        .arg("-y");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("All jobs deleted successfully."));

    // After running binary, `bk list` should show no jobs (empty output)
    let mut list2 = Command::new(&bin);
    list2.env("XDG_CONFIG_HOME", temp.path()).arg("list");
    list2
        .assert()
        .success()
        .stdout(predicate::str::contains("id:").not());

    Ok(())
}
