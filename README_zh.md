# hbackup

hbackup 是一个用 Rust 编写的高性能跨平台备份工具。它以快速、高效、易用为设计理念，注重性能和可靠性。

## 功能特点

- 通过命令行快速备份文件或目录
- 跨平台支持：macOS、Linux、Windows
- 支持自定义备份任务并分配唯一ID
- 配置和任务管理存储于用户配置目录的 JSON 文件
- 支持 `~` 路径自动展开

## 快速上手

### 1. 安装

```sh
cargo install hbackup --version 0.1.0-beta.2
```

### 2. 创建备份任务

```sh
bk create --source ~/myfile.txt --target ~/backup/
```

或指定任务ID：

```sh
bk create --source ~/myfile.txt --target ~/backup/ --id 1
```

### 3. 执行所有备份任务

```sh
bk run
```

### 4. 查看所有任务

```sh
bk list
```

### 5. 删除任务

按ID删除：

```sh
bk delete --id 1
```

删除全部任务：

```sh
bk delete --all
```

## 配置文件位置

- macOS/Linux: `~/.config/hbackup/tasks.json`
- Windows: `C:\Users\<User>\AppData\Roaming\hbackup\tasks.json`

## 许可证

MIT OR Apache-2.0
