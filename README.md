# hbackup

hbackup is a high-performance, cross-platform CLI backup tool written in Rust.

## Features

- Simple and fast file/directory backup via CLI
- Cross-platform: macOS, Linux, Windows
- Supports custom backup tasks with unique IDs
- Configuration and task management via JSON in user config directory
- Supports `~` path expansion

## Quick Start

### 1. Install

```sh
cargo install hbackup --version 0.1.0-beta.0
```

### 2. Create a backup task

```sh
hbackup create --source ~/myfile.txt --target ~/backup/
```

Or specify a task id:

```sh
hbackup create --source ~/myfile.txt --target ~/backup/ --id 1
```

### 3. Run all backup tasks

```sh
hbackup run
```

### 4. List all tasks

```sh
hbackup list
```

### 5. Delete a task

Delete by id:

```sh
hbackup delete --id 1
```

Delete all tasks:

```sh
hbackup delete --all
```

## Configuration File Location

- macOS/Linux: `~/.config/hbackup/tasks.json`
- Windows: `C:\Users\<User>\AppData\Roaming\hbackup\tasks.json`

## License

MIT OR Apache-2.0
