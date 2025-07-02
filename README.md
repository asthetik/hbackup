# hbackup

[![Build status](https://github.com/asthetik/hbackup/workflows/build/badge.svg)](https://github.com/asthetik/hbackup/actions)
[![Crates.io](https://img.shields.io/crates/v/hbackup.svg)](https://crates.io/crates/hbackup)
![Crates.io](https://img.shields.io/crates/d/hbackup)
[![MIT License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

[English](./README.md) | [简体中文](./README.zh-CN.md)

**hbackup** is a simple, high-performance, cross-platform backup tool written in Rust. It is designed to be fast, efficient, and easy to use, with a focus on performance, reliability, and flexible backup management.

---

## Features

- 🚀 **Fast and simple** file/directory backup via CLI
- 🖥️ **Cross-platform**: macOS, Linux, Windows
- 🗂️ **Custom backup jobs** with unique IDs
- 📝 **Configuration and task management** via TOML in user config directory
- 🏠 Supports `~`, `$HOME`, and relative paths for source and target
- 🔄 **Edit, delete, and list** backup jobs easily
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
bk add --source ~/my_path1/my_file1.txt --target ~/back
bk add --source ~/my_path2/my_file2.txt --target ~/back
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

- **Run a job by ID:**
  
  ```sh
  bk run --id 1
  ```

- **Run a one-time backup (without saving as a job):**
  
  ```sh
  bk run ~/my_path/myfile.txt ~/back
  ```

### 5. Delete jobs

- **Delete a job by ID:**

  ```sh
  bk delete --id 1
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

- **macOS/Linux:** `~/.config/hbackup/hbackup.json`
- **Windows:** `C:\Users\<User>\AppData\Roaming\hbackup\hbackup_backup.json`

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
