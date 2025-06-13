# hbackup

[![Crates.io](https://img.shields.io/crates/v/hbackup.svg)](https://crates.io/crates/hbackup)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

[English](./README.md) | [简体中文](./README.zh-CN.md)

hbackup is a sample, high-performance, cross-platform backup tool written in Rust. It is designed to be fast, efficient, and easy to use, with a focus on performance and reliability.

## Features

- Simple and fast file/directory backup via CLI
- Cross-platform: macOS, Linux, Windows
- Supports custom backup tasks with unique IDs
- Configuration and task management via JSON in user config directory
- Supports `~` path expansion

## Quick Start

### 1. Install

```sh
cargo install hbackup --version 0.1.0-beta.7
```

### 2. Add a backup task

```sh
bk add --source ~/myfile.txt --target ~/backup/
```

### 3. Run all backup tasks

- run all tasks:

```sh
bk run
```

- run a specific task by ID:

```sh
bk run --id 1
```

- run a specific task with source and target:

```sh
bk run ~/myfile.txt ~/backup/
```

### 4. List all tasks

```sh
bk list
```

### 5. Delete a task

Delete by id:

```sh
bk delete --id 1
```

Delete all tasks:

```sh
bk delete --all
```

### 6. Edit a task

```sh
bk edit --id 1 --source ~/newfile.txt --target ~/newbackup/
```

### 7. Display configuration file path

```shell
bk config
```

## Configuration File Location

- macOS/Linux: `~/.config/hbackup/hbackup.json`
- Windows: `C:\Users\<User>\AppData\Roaming\hbackup\hbackup.json`

## License

MIT OR Apache-2.0
