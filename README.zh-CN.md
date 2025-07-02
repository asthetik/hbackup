# hbackup

[![Build status](https://github.com/asthetik/hbackup/workflows/build/badge.svg)](https://github.com/asthetik/hbackup/actions)
[![Crates.io](https://img.shields.io/crates/v/hbackup.svg)](https://crates.io/crates/hbackup)
![Crates.io](https://img.shields.io/crates/d/hbackup)
[![MIT License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

[English](./README.md) | [简体中文](./README.zh-CN.md)

**hbackup** 是一个用 Rust 编写的高性能、跨平台备份工具。它专注于速度、效率、易用性和灵活的备份管理。

---

## 功能特点

- 🚀 **快速简单** 的文件/目录备份 CLI 工具
- 🖥️ **跨平台**：macOS、Linux、Windows
- 🗂️ **自定义备份任务**，每个任务有唯一 ID
- 📝 **配置和任务管理**，基于用户配置目录下的 TOML 文件
- 🏠 支持 `~`、`$HOME` 和相对路径作为源和目标
- 🔄 **轻松编辑、删除、列出**备份任务
- 🗜️ **压缩支持**：文件和目录均可用 `gzip`, `zip`, `sevenz`, `zstd` 格式压缩
- 🛠️ **配置文件备份、重置与回滚**
- 📦 **一次性备份**：无需保存任务即可执行备份
- 🧩 **易扩展**，方便添加新功能

---

## 快速上手

### 1. 安装

```sh
cargo install hbackup
```

### 2. 添加一个或多个备份任务

```sh
bk add --source ~/my_path1/my_file1.txt --target ~/back
bk add --source ~/my_path2/my_file2.txt --target ~/back
# 添加带压缩的任务（gzip 或 zip）
bk add -s ~/my_path3/my_dir -t ~/back -c gzip
bk add -s ~/my_path4/my_dir -t ~/back -c zip
```

### 3. 查看所有任务

```sh
bk list
```

### 4. 执行备份任务

- **执行所有任务：**
  
  ```sh
  bk run
  ```

- **按 ID 执行任务：**
  
  ```sh
  bk run --id 1
  ```

- **一次性备份（不保存为任务）：**
  
  ```sh
  bk run ~/my_path/myfile.txt ~/back
  ```

  也可以指定压缩格式：

  ```sh
  bk run ~/my_path/mydir ~/back --compression gzip
  bk run ~/my_path/mydir ~/back --compression zip
  ```

### 5. 删除任务

- **按 ID 删除任务：**

  ```sh
  bk delete --id 1
  ```

- **删除全部任务：**
  
  ```sh
  bk delete --all
  ```

### 6. 编辑任务

根据任务 ID 更新源和/或目标路径：

```sh
bk edit --id 1 --source ~/newfile.txt --target ~/newbackup/
```

### 7. 配置文件管理

- **显示配置文件路径：**

  ```sh
  bk config
  ```

- **备份配置文件：**

  ```sh
  bk config --copy
  ```

- **重置配置文件（重置前自动备份）：**

  ```sh
  bk config --reset
  ```

- **回滚到上一次备份的配置文件：**

  ```sh
  bk config --rollback
  ```

---

## 压缩支持

你可以在**添加任务**或**运行任务**时指定压缩格式（`gzip`, `zip`, `sevenz` 或 `zstd`）：

```sh
# 添加带 gzip 压缩的任务
bk add --source ~/file.txt --target ~/back --compression gzip
# 或者简短的命令行
bk add -s ~/file.txt -t ~/back -c gzip

# 一次性备份并压缩
bk run ~/my_path/mydir ~/back gzip
```

- 压缩支持文件和目录。
- 输出文件会有 `gz`, `zip`, `7z` 或 `zst`  扩展名。
- 如果未指定压缩，则直接复制文件。

---

## 命令参考

| 命令                    | 说明                                   |
|-------------------------|----------------------------------------|
| `bk add`                | 添加新的备份任务                        |
| `bk list`               | 列出所有备份任务                        |
| `bk run`                | 执行所有任务、指定 ID 或一次性备份       |
| `bk delete`             | 按 ID 删除任务或删除全部任务            |
| `bk edit`               | 按 ID 编辑任务的源/目标路径             |
| `bk config`             | 显示、备份、重置或回滚配置文件          |

使用 `bk <命令> --help` 查看详细参数。

---

## 配置文件位置

- **macOS/Linux:** `~/.config/hbackup/config.toml`
- **Windows:** `C:\Users\<User>\AppData\Roaming\hbackup\config.toml`

重置配置文件前会自动备份。

---

## 错误处理

- 所有错误均有清晰提示。
- 如果你直接运行 `bk`，会看到：

```sh
error: hbackup requires at least one command to execute.

See 'bk --help' for usage.
```

---

## 许可证

MIT
