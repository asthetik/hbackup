# hbackup

[![Build status](https://github.com/asthetik/hbackup/workflows/build/badge.svg)](https://github.com/asthetik/hbackup/actions)
[![Crates.io](https://img.shields.io/crates/v/hbackup.svg)](https://crates.io/crates/hbackup)
![Crates.io](https://img.shields.io/crates/d/hbackup)
[![MIT License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

[English](./README.md) | [简体中文](./README.zh-CN.md)

hbackup is a simple, high-performance, cross-platform backup tool written in Rust. It is designed to be fast, efficient, and easy to use, with a focus on performance and reliability.

## Features

- Simple and fast file/directory backup via CLI
- Cross-platform: macOS, Linux, Windows
- Supports custom backup tasks with unique IDs
- Configuration and task management via toml in user config directory
- Supports '~', '$HOME' and relative paths for source and target paths

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

### 4. Run all backed jobs

- run all jobs:

```sh
bk run
```

- Run the job with the specified ID:

```sh
bk run --id 1
```

- run a specific job with source and target:

```sh
bk run ~/my_path/myfile.txt ~/back
```

### 5. Delete a job

- Delete a job by id:

```sh
bk delete --id 1
```

- Delete all jobs:

```sh
bk delete --all
```

### 6. Edit a job

```sh
bk edit --id 1 --source ~/newfile.txt --target ~/newbackup/
```

### 7. configuration file

display configuration file path

```shell
bk config
```

- backup configutation file

```sh
bk config --copy
```

- reset configuration file (The file will be automatically backed up before resetting it)

```sh
bk config --reset
```

- Rollback the last backed up configuration file

```sh
bk config --rollback
```

## Configuration File Location

- macOS/Linux: `~/.config/hbackup/config.toml`
- Windows: `C:\Users\<User>\AppData\Roaming\hbackup\config.toml`

## License

MIT
