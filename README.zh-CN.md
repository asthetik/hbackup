# hbackup

[![Build status](https://github.com/asthetik/hbackup/workflows/build/badge.svg)](https://github.com/asthetik/hbackup/actions)
[![Crates.io](https://img.shields.io/crates/v/hbackup.svg)](https://crates.io/crates/hbackup)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](./LICENSE)

[English](./README.md) | [简体中文](./README.zh-CN.md)

hbackup 是一个用 Rust 编写的高性能跨平台备份工具。它以快速、高效、易用为设计理念，注重性能和可靠性。

## 功能特点

- 通过命令行快速备份文件或目录
- 跨平台支持：macOS、Linux、Windows
- 支持自定义备份任务并分配唯一ID
- 配置和任务管理存储于用户配置目录的 JSON 文件
- 支持 `~` 、`$HOME` 和相对路径作为源和目标路径

## 快速上手

### 1. 安装

```sh
cargo install hbackup
```

### 2. 添加一个或多个备份任务

```sh
bk add --source ~/my_path1/my_file1.txt --target ~/back
bk add --source ~/my_path2/my_file2.txt --target ~/back
```

### 3. 查看所有任务

```sh
bk list
```

### 4. 执行所有任务

- 运行所有任务：

```sh
bk run
```

- 运行指定ID的任务：

```sh
bk run --id 1
```

- 运行指定源和目标的任务：

```sh
bk run ~/my_path/myfile.txt ~/back
```

### 5. 删除任务

- 按ID删除：

```sh
bk delete --id 1
```

- 删除全部任务：

```sh
bk delete --all
```

### 6. 编辑任务

```sh
bk edit --id 1 --source ~/newfile.txt --target ~/newbackup/
```

### 7. 配置文件

- 显示配置文件路径

```sh
bk config
```

- 备份配置文件

```sh
bk config --copy
```

- 重置备份文件（重置文件之前会自动备份配置文件）

```sh
bk config --reset
```

- 回滚上一次备份的配置文件

```sh
bk config --rollback
```

## 配置文件位置

- macOS/Linux: `~/.config/hbackup/hbackup.json`
- Windows: `C:\Users\<User>\AppData\Roaming\hbackup\hbackup.json`

## 许可证

MIT OR Apache-2.0
