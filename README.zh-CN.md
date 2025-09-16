# hbackup

[![CI](https://github.com/asthetik/hbackup/workflows/CI/badge.svg)](https://github.com/asthetik/hbackup/actions/workflows/ci.yml)
[![Security](https://github.com/asthetik/hbackup/workflows/Security/badge.svg)](https://github.com/asthetik/hbackup/actions/workflows/security.yml)
[![Crates.io](https://img.shields.io/crates/v/hbackup.svg)](https://crates.io/crates/hbackup)
![Crates.io](https://img.shields.io/crates/d/hbackup)
[![MIT License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

[English](./README.md) | [简体中文](./README.zh-CN.md)

**hbackup** 是一个用 Rust 编写的高性能、跨平台备份工具。它专注于速度、效率、易用性和灵活的备份管理。

---

## 功能特点

- 🚀 **快速简单** 的文件/目录备份 CLI 工具
- ⚡️ **异步多线程备份**，大文件或多文件场景下性能更高
- 🖥️ **跨平台**：macOS、Linux、Windows
- 📝 **配置和任务管理**，基于用户配置目录下的 TOML 文件
- 🔄 **轻松添加、编辑、删除、列出和运行**备份任务
- 🗜️ **压缩支持**：文件和目录均可用 `gzip`, `zip`, `sevenz`, `zstd`, `bzip2`, `xz`, `lz4`, `tar` 格式压缩
- 🛠️ **配置文件备份、重置与回滚**
- 🧩 **易扩展**，方便添加新功能

---

## 快速上手

### 1. 安装

```sh
cargo install hbackup
```

### 2. 添加一个或多个备份任务

```sh
bk add ~/my_path1/my_file1.txt ~/back
# 添加带压缩的任务
bk add ~/my_path2/my_dir ~/back -c gzip
bk add ~/my_path3/my_dir ~/back -c zip -l best
# 添加带镜像（删除目标中源不存在的文件）的任务
bk add ~/my_path4/my_dir ~/back -m mirror
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

- **按 ID 执行多个任务：**
  
  ```sh
  bk run -i 1
  bk run -i 1,2
  # 或使用完整形式
  bk run --id 1,2
  ```

- **一次性备份（不保存为任务）：**
  
  ```sh
  bk run ~/my_path/myfile.txt ~/back
  ```

  也可以指定压缩格式：

  ```sh
  bk run ~/my_path/mydir ~/back -c gzip
  bk run ~/my_path/mydir ~/back -c zip -l best
  ```

### 5. 删除任务

- **按 ID 删除多个任务：**

  ```sh
  bk delete 1
  bk delete 1,2
  ```

- **删除全部任务：**
  
  ```sh
  bk delete -a
  # 或使用完整形式
  bk delete --all
  ```

### 6. 编辑任务

根据任务 ID 更新源和/或目标路径：

```sh
bk edit 1 --source ~/newfile.txt --target ~/newbackup/
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

你可以在**添加任务**或**运行任务**时指定压缩格式（`gzip`, `zip`, `sevenz`, `zstd`, `bzip2`, `xz`, `lz4`, `tar`）：

```sh
# 添加带 gzip 压缩的任务
bk add ~/file.txt ~/back --compression gzip --level fastest
# 或者简短的命令行
bk add ~/file.txt ~/back -c gzip -l fastest

# 一次性备份并压缩
bk run ~/my_path/mydir ~/back -c gzip
```

- 压缩支持文件和目录。
- 输出文件会有 `gz`, `zip`, `7z`, `zst`, `bzip2`, `xz`, `lz4`, `tar` 扩展名。
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

```text
bk requires at least one command to execute. See 'bk --help' for usage.
```

---

## 许可证

MIT
