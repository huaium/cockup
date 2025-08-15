# cockup

![PyPI - Version](https://img.shields.io/pypi/v/cockup?link=https%3A%2F%2Fpypi.org%2Fproject%2Fcockup%2F)

[English](README.md) | 中文

又一个用于备份各种配置文件的工具。

## 安装

### PyPI

```bash
pip install cockup
```

### Homebrew

Homebrew 在构建部分依赖时会安装 Rust 工具链，可能会导致过长的安装时间，建议优先考虑通过 PyPI 安装。

```bash
# 单行命令
brew install huaium/tap/cockup

# 或者使用 `brew tap`
brew tap huaium/tap
brew install cockup
```

### 从源码安装

1. 克隆或下载本项目
2. 切换到项目根目录，运行：

```bash
pip install -e .
```

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
- from: "/source/directory"
  targets:
    # 在 `from` 目录下的文件夹或文件
    # 允许使用通配符匹配
    - "pattern1"
    - "pattern2"
  to: "backup/subdirectory" # 在 `destination` 下的子文件夹
  on-start: # 规则层面的可选 Hooks
    - name: "Before Rule"
      command: ["echo", "Processing rule"]
  on-end:
    - name: "After Rule"
      command: ["echo", "Rule complete"]
```

### Hook 结构

Hooks 允许用户自定义运行命令。

默认情况下，如果你的配置文件包含任何 Hooks，程序会在执行命令前要求手动确认。使用 `--quiet` 或 `-q` 标志可以规避该行为。

如果你需要在一个特定的 Shell 里面运行命令，则请务必在检查安全性之后再使用诸如 `bash -c` 的命令来运行。

```yaml
- name: "Hook Name" # 必要：作为 Hook 的标识符
  command: ["cmd", "arg1"] # 必要：命令参数列表
  output: false # 可选：显示命令输出 (default: false)
  timeout: 10 # 可选：允许运行秒数 (default: 10)
```

一个典型的场景是备份 Homebrew bundle 列表，生成的文件将放置在 `destination` 指定的文件夹下：

```yaml
- name: "Brewfile Dumping"
  command: ["brew", "bundle", "dump", "--force", "--file", "Brewfile"]
  output: true
  timeout: 10
```

请访问 [sample](sample) 查看配置用例。

## 开发

本项目使用 `just` 来标准化开发工作流。如果你不打算使用它，请查看位于项目根目录的 `justfile` 来获取原始命令。

### 安装测试依赖

使用 `pytest` 作为测试框架。

```bash
just install-test
```

### 直接运行

通过 `just run [ARGS]`。

```bash
# `cockup list`
just run list

# `cockup backup`
just run backup /path/to/config.yaml
```

### 运行用例

[sample](sample) 是一个最小化用例，可用于手动运行测试。

```bash
# 测试 `cockup backup`
just sample-backup

# 或者测试 `cockup restore`
just sample-restore
```

### 测试

`just test` 可作为 `pytest` 的别名。

```bash
# 运行所有测试
just test

# 运行所有测试并生成 Coverage
just test --cov=cockup

# 运行指定测试
just test tests/test_config.py -v
```

### 构建

`just build` 可作为 `uv build` 的别名。

## 许可证

```
MIT License

Copyright (c) 2025 Huaium

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
