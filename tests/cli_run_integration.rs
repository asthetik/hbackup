//! CLI integration tests for `bk run` exit codes and non-existent backup targets.
//!
//! Config isolation: `HBACKUP_CONFIG` overrides the config directory on every
//! platform, so tests never touch the real user config (on Windows the
//! default path ignores HOME/XDG env vars entirely).

use assert_cmd::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use std::process::Command;

/// Returns a `bk` command isolated from the real user config.
fn bk(temp: &TempDir) -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("bk"));
    cmd.env("HBACKUP_CONFIG", temp.path());
    cmd
}

/// Adds two gzip jobs: a good one (id 1) and one whose target is a regular
/// file (id 2), which is guaranteed to fail at run time because compression
/// requires a directory target. Having two jobs forces `bk run` down the
/// multi-job (`run_jobs`) path.
fn add_good_and_bad_jobs(temp: &TempDir) -> anyhow::Result<()> {
    let src = temp.path().join("src.txt");
    std::fs::write(&src, "hello")?;
    let good_target = temp.path().join("backup");
    std::fs::create_dir_all(&good_target)?;
    bk(temp)
        .args(["add"])
        .arg(&src)
        .arg(&good_target)
        .arg("-c")
        .arg("gzip")
        .assert()
        .success();

    let src2 = temp.path().join("src2.txt");
    std::fs::write(&src2, "world")?;
    let file_target = temp.path().join("not_a_dir");
    std::fs::write(&file_target, "occupied")?;
    bk(temp)
        .args(["add"])
        .arg(&src2)
        .arg(&file_target)
        .arg("-c")
        .arg("gzip")
        .assert()
        .success();

    Ok(())
}

#[test]
fn run_all_exits_non_zero_when_any_job_fails() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    add_good_and_bad_jobs(&temp)?;

    bk(&temp)
        .args(["run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to run job with id 2"));

    // The good job still ran despite its sibling failing.
    let archive = temp.path().join("backup").join("src.txt.gz");
    assert!(archive.exists(), "good job should still produce its backup");

    Ok(())
}

#[test]
fn run_single_failing_job_exits_non_zero() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    add_good_and_bad_jobs(&temp)?;

    // Same exit code as the multi-job `bk run` path: failures always exit 1.
    bk(&temp)
        .args(["run", "-i", "2"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Failed to run job with id 2"));

    Ok(())
}

#[test]
fn empty_hbackup_config_env_falls_back_to_absolute_default() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    // An empty override must be ignored instead of resolving the config to
    // a relative ./config.toml inside the working directory. HOME (and
    // XDG_CONFIG_HOME) redirect the fallback default on Unix; Windows uses
    // its known-folder API, which is absolute by construction — there the
    // fallback is the real config dir, and `bk config` only creates the
    // file if it is missing (never overwrites).
    Command::new(assert_cmd::cargo::cargo_bin!("bk"))
        .arg("config")
        .env("HBACKUP_CONFIG", "")
        .env("HOME", temp.path())
        .env("XDG_CONFIG_HOME", temp.path().join(".config"))
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::function(|out: &str| {
            std::path::Path::new(out.trim()).is_absolute()
        }));
    Ok(())
}

#[test]
fn run_all_with_only_good_jobs_exits_zero() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let src1 = temp.path().join("a.txt");
    std::fs::write(&src1, "1")?;
    let src2 = temp.path().join("b.txt");
    std::fs::write(&src2, "2")?;
    let tgt1 = temp.path().join("t1");
    std::fs::create_dir_all(&tgt1)?;
    let tgt2 = temp.path().join("t2");
    std::fs::create_dir_all(&tgt2)?;
    bk(&temp)
        .args(["add"])
        .arg(&src1)
        .arg(&tgt1)
        .arg("-c")
        .arg("gzip")
        .assert()
        .success();
    bk(&temp)
        .args(["add"])
        .arg(&src2)
        .arg(&tgt2)
        .arg("-c")
        .arg("gzip")
        .assert()
        .success();

    bk(&temp).args(["run"]).assert().success();

    Ok(())
}

#[test]
fn one_shot_backup_to_new_target_dir_succeeds() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let src = temp.path().join("doc.txt");
    std::fs::write(&src, "data")?;
    let target = temp.path().join("fresh_target");

    bk(&temp)
        .args(["run"])
        .arg(&src)
        .arg(&target)
        .assert()
        .success();

    let backed_up = target.join("doc.txt");
    assert!(
        backed_up.exists(),
        "file should land inside the new target dir"
    );
    assert_eq!(std::fs::read_to_string(backed_up)?, "data");
    Ok(())
}

#[test]
fn dir_backup_to_new_target_dir_succeeds() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let src_dir = temp.path().join("mydata");
    std::fs::create_dir_all(&src_dir)?;
    std::fs::write(src_dir.join("in.txt"), "content")?;
    let target = temp.path().join("fresh_target");

    bk(&temp)
        .args(["run"])
        .arg(&src_dir)
        .arg(&target)
        .assert()
        .success();

    assert!(target.join("mydata").join("in.txt").exists());
    Ok(())
}

#[test]
fn add_job_with_new_target_dir_succeeds() -> anyhow::Result<()> {
    let temp = TempDir::new()?;
    let src = temp.path().join("s.txt");
    std::fs::write(&src, "x")?;
    let target = temp.path().join("not_yet_created");

    bk(&temp)
        .args(["add"])
        .arg(&src)
        .arg(&target)
        .assert()
        .success();

    bk(&temp)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not_yet_created"));

    Ok(())
}
