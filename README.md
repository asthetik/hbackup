# hbackup

[![CI](https://github.com/asthetik/hbackup/workflows/CI/badge.svg)](https://github.com/asthetik/hbackup/actions/workflows/ci.yml)
[![Security](https://github.com/asthetik/hbackup/workflows/Security/badge.svg)](https://github.com/asthetik/hbackup/actions/workflows/security.yml)
[![Crates.io](https://img.shields.io/crates/v/hbackup.svg)](https://crates.io/crates/hbackup)
![Crates.io](https://img.shields.io/crates/d/hbackup)
[![MIT License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

[English](./README.md) | [简体中文](./README.zh-CN.md)

**hbackup** is a simple, high-performance, cross-platform backup tool written in Rust. It is designed to be fast, efficient, and easy to use, with a focus on performance, reliability, and flexible backup management.

---

## Features

- 🚀 **Fast and simple** file/directory backup via CLI
- ⚡️ **Asynchronous multi-threaded backup** for higher performance, especially with large or multiple files
- 🖥️ **Cross-platform**: macOS, Linux, Windows
- 🗂️ **Custom backup jobs** with unique IDs
- 📝 **Configuration and task management** via TOML in user config directory
- 🏠 Supports `~`, `$HOME`, and relative paths for source and target
- 🔄 **Edit, delete, and list** backup jobs easily
- 🗜️ **Compression support**: `gzip`, `zip`, `sevenz`, `zstd`, `bzip2`, `xz` and `lz4` for files and directories
- 🛠️ **Config file backup, reset, and rollback**
- 📦 **One-time backup**: run a backup without saving a job
- 🧩 **Extensible**: easy to add new features

---

## Quick Start

### 1. Install

```sh
cargo install hbackup
```

### 2. Add one or more jobs

```sh
bk add ~/my_path1/my_file1.txt ~/back
# Add a job with compression
bk add ~/my_path3/my_dir ~/back -c gzip
bk add ~/my_path4/my_dir ~/back -c zip -l best
```

### 3. List all jobs

```sh
bk list
```

### 4. Run backup jobs

- **Run all jobs:**
  
  ```sh
  bk run
  ```

- **Run multiple jobs by ID:**
  
  ```sh
  bk run --id 1
  bk run --id 1,2
  ```

- **Run a one-time backup (without saving as a job):**
  
  ```sh
  bk run ~/my_path/myfile.txt ~/back
  ```

  You can also specify compression for a one-time backup:

  ```sh
  bk run ~/my_path/mydir ~/back -c gzip
  bk run ~/my_path/mydir ~/back -c zip -l best
  ```

### 5. Delete jobs

- **Delete multiple jobs by ID:**

  ```sh
  bk delete --id 1
  bk delete --id 1,2
  ```

- **Delete all jobs:**
  
  ```sh
  bk delete --all
  ```

### 6. Edit a job

Update the source and/or target of a job by its ID:

```sh
bk edit --id 1 --source ~/newfile.txt --target ~/newbackup/
```

### 7. Manage configuration file

- **Show configuration file path:**

  ```sh
  bk config
  ```

- **Backup configuration file:**

  ```sh
  bk config --copy
  ```

- **Reset configuration file (auto-backup before reset):**

  ```sh
  bk config --reset
  ```

- **Rollback to the last backed up configuration file:**

  ```sh
  bk config --rollback
  ```

---

## Compression Support

You can specify compression format (`gzip`, `zip`, `sevenz`, `zstd`, `bzip2` or `lz4`) when **adding** or **running** jobs:

```sh
# Add a job with gzip compression
bk add ~/file.txt ~/back --compression gzip --level fastest
# or a short command line
bk add ~/file.txt ~/back -c gzip -l fastest

# One-time backup with compression
bk run ~/my_path/mydir ~/back -c gzip
```

- Compression works for both files and directories.
- Output files will have `gz`, `zip`, `7z`, `zst`, `bzip2`, `xz` or `lz4` extensions.
- If no compression is specified, files are copied as-is.

---

## Command Reference

| Command                | Description                                      |
|------------------------|--------------------------------------------------|
| `bk add`               | Add a new backup job                             |
| `bk list`              | List all backup jobs                             |
| `bk run`               | Run all jobs, a job by ID, or a one-time backup  |
| `bk delete`            | Delete a job by ID or delete all jobs            |
| `bk edit`              | Edit a job's source/target by ID                 |
| `bk config`            | Show, backup, reset, or rollback config file     |

Run `bk <command> --help` for detailed options.

---

## Configuration File Location

- **macOS/Linux:** `~/.config/hbackup/config.toml`
- **Windows:** `C:\Users\<User>\AppData\Roaming\hbackup\config.toml`

A backup of the config file is automatically created before resetting.

---

## Error Handling

- All errors are reported with clear messages.
- If you run `bk` without a command, you'll see:

```sh
  error: hbackup requires at least one command to execute.

  See 'bk --help' for usage.
```

---

## License

MIT
