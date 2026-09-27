# cockup

[![CI](https://github.com/huaium/cockup/actions/workflows/rust.yml/badge.svg)](https://github.com/huaium/cockup/actions/workflows/rust.yml)

[English](README.md) | 中文

又一个用于备份各种配置文件的工具。

## 安装

Cockup 0.2.0 改用 Rust，支持 Apple Silicon 和 Intel macOS，无需 Python。

发布后可从 [GitHub Releases](https://github.com/huaium/cockup/releases) 下载对应架构的
压缩包，校验 SHA256 后解压，将 `cockup` 放入 `PATH` 中的目录。
也可在 crates.io 发布后安装：

```sh
cargo install cockup --locked
```

从源码构建：

```sh
cargo build --release --locked
./target/release/cockup --help
```

旧版 PyPI 包仍然可用，但不会安装 Rust 版本。现有 Homebrew tap 需要单独更新。

## 使用

### `cockup list`

你也许会想将它作为编写备份规则的参考。

```bash
# 列出所有已安装的 Homebrew casks 可能存在的配置路径
cockup list

# 列出指定 cask 可能存在的配置路径
cockup list cask-name-1 [cask-name-n...]
```

### `cockup backup & restore`

```bash
# 依据指定的配置规则进行备份
cockup backup /path/to/config.yaml

# 从备份恢复
cockup restore /path/to/config.yaml
```

### `cockup hook`

```bash
# 交互式地运行所选中的 Hooks
cockup hook /path/to/config.yaml

# 或者根据 name 运行指定的 Hook
cockup hook /path/to/config.yaml --name hook_name
```

## 配置

创建一个遵循以下结构的 YAML 配置文件：

### 必要字段

```yaml
# 备份文件的存储位置
# 如果你打算使用相对路径，则请务必注意是相对于该配置文件的路径
destination: "/path/to/backup/directory"

# 备份规则列表
rules:
  - from: "/source/directory"
    targets: ["*.conf", "*.json"]
    to: "subdirectory"
```

### 可选字段

```yaml
# 导入其他配置文件中的规则和 Hooks
include:
  - "./another_config.yaml"

# 清洁模式，即是否先删除现有备份 (default: false)
clean: false

# 是否在备份时保留元数据 (default: true)
metadata: true

# 全局 Hooks
hooks:
  pre-backup:
    - name: "Setup"
      command: ["echo", "Starting backup"]
  post-backup:
    - name: "Cleanup"
      command: ["echo", "Backup complete"]
  pre-restore:
    - name: "Prepare"
      command: ["echo", "Starting restore"]
  post-restore:
    - name: "Finish"
      command: ["echo", "Restore complete"]
```

### 规则结构

每条规则都定义了程序所要备份的内容：

```yaml
- from: "/source/directory/in/pattern" # 允许使用通配符匹配
  targets:
    # 在 `from` 目录下的文件夹或文件
    - "pattern1" # 同样允许使用通配符匹配
    - "relative/path/to/file"
  to: "backup/subdirectory" # 在 `destination` 下的子文件夹
  on-start: # 规则层面的可选 Hooks
    - name: "Before Rule"
      command: ["echo", "Processing rule"]
  on-end:
    - name: "After Rule"
      command: ["echo", "Rule complete"]
```

请注意，当你在 `from` 中使用通配符时，为避免匹配多个目录导致的文件重复，cockup 会向上寻找第一个不存在通配符的安全目录，并将其设置为 `from`。而寻找过程中经过的包含通配符的路径，会拼接在每个 `target` 前。

例如，假设有如下的配置文件：

```yaml
- from: "/source/directory/in/prefix*/pattern"
  targets:
    - "relative/path/to/file"
  to: "backup/subdirectory"
```

我们最终会得到：

```yaml
- from: "/source/directory/in"
  targets:
    - "prefix*/pattern/relative/path/to/file"
  to: "backup/subdirectory"
```

### Hook 结构

Hooks 允许用户自定义运行命令。

默认情况下，如果你的配置文件包含任何 Hooks，程序会在执行命令前要求手动确认。使用 `--quiet` 或 `-q` 标志可以规避该行为。

如果你需要在一个特定的 Shell 里面运行命令，则请务必在检查安全性之后再使用诸如 `bash -c` 的命令来运行。

```yaml
- name: "Hook Name" # 必要：作为 Hook 的标识符
  command: ["cmd", "arg1"] # 必要：命令参数列表
  output: false # 可选：显示命令输出 (default: false)
  timeout: 10 # 可选：允许运行秒数
  env: # 可选: 用于该命令的环境变量
    ENV_1: 1
    ENV_2: 2
```

请注意，你无法直接使用类似 `$ENV_1` 的语法传递环境变量，但所启动的子进程能够访问这些变量。

一个典型的场景是备份 Homebrew bundle 列表，生成的文件将放置在 `destination` 指定的文件夹下：

```yaml
- name: "Brewfile Dumping"
  command: ["brew", "bundle", "dump", "--force", "--file", "Brewfile"]
  check: ["test", "-s", "Brewfile"]
  output: true
  timeout: 10
```

可选的 `check` 命令在 `command` 成功退出后执行，两者退出码都为 0 才算 Hook 成功。
检查命令使用相同的工作目录、`env` 和 `output`，`timeout` 分别作用于两个命令。
检查失败或超时会报告错误，但仍会继续执行后续 Hooks。

### 配置导入

你可以简单地通过 `include` 语句导入其他配置文件中的规则和 Hooks。

请注意，仅有规则和 Hooks 会被导入，且它们将被置于主配置中定义的规则和 Hooks 之前，执行时将使用主配置中的 `clean` 和 `metadata` 字段。

```yaml
include:
  - "path_to/config_one.yaml"
  - "path_to/config_two.yaml"
```

请访问 [sample](sample) 查看配置用例。

## 开发

使用原生 macOS 和支持 Rust 2024 的 Rust 工具链，无需虚拟环境。

```sh
cargo run --locked -- list
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
```

`just` 显示所有命令；`just run`、`just test`、`just build` 和样例命令调用 Cargo。
测试通过 CLI 和临时文件验证行为，使用可执行脚本模拟 Homebrew 和 Hooks。
真实的 Homebrew 验证可运行 `cargo run --locked -- list iterm2`。

## 从 Python 迁移

命令和 YAML 字段保持兼容。相对路径以各自配置文件所在目录为准，循环导入和无效配置会在执行前报错。
Hooks 只确认一次，`--quiet`（或 `--yes`）同时作用于导入的 Hooks。

成功或拒绝确认返回 0；配置、复制、Hook 或 Homebrew 错误返回 1；命令行用法错误返回 2。
可恢复的错误不会阻止后续操作，但最终摘要显示失败。特殊文件和不存在的普通目标仍会跳过并提示。
元数据模式保留文件权限、时间戳和受支持的 macOS 文件标志，不保证 ACL、所有者或扩展属性。

参见 [迁移验证记录](docs/migration.md) 和 [样例](sample/README.md)。

## 许可证

请查阅 [LICENSE](./LICENSE)。
