# cockup

[![CI](https://github.com/huaium/cockup/actions/workflows/rust.yml/badge.svg)](https://github.com/huaium/cockup/actions/workflows/rust.yml)

[English](README.md) | 中文

又一个用于备份各种配置文件的工具。

## 安装

通过 [Homebrew](https://github.com/huaium/homebrew-tap) 安装：

```sh
brew install huaium/tap/cockup
```

或使用 Cargo 从 [crates.io](https://crates.io/crates/cockup) 安装：

```sh
cargo install cockup --locked
```

预编译二进制文件也可从 [GitHub Releases](https://github.com/huaium/cockup/releases) 下载。
源码构建参见[开发指南](docs/zh-cn/development.md)。

## 典型用例

保存以下内容为 `config.yaml`，将 Shell 和 Git 配置备份到 YAML 文件旁的 `backup` 目录：

```yaml
destination: ./backup
symlinks: reference
rules:
  - from: "~"
    targets: [".zshrc", ".gitconfig"]
    to: dotfiles
```

预览变更、创建备份，并在需要时恢复：

```sh
cockup backup config.yaml --dry-run
cockup backup config.yaml
cockup restore config.yaml
```

恢复时会显示差异，并在覆盖冲突的本地文件前询问。

## 命令

使用 `--dry-run` 预览变更，使用 `diff` 比较本地文件，使用 `verify` 检查备份完整性。
Cockup 还支持 Hooks 和 Homebrew 配置发现。详情参见[命令参考](docs/zh-cn/commands.md)。

## 配置

通过 YAML 规则选择文件及其备份路径，并按需配置符号链接处理、元数据保留、Hooks 和可复用的导入。
所有字段参见[配置参考](docs/zh-cn/configuration.md)。

## 配料

导入预定义的应用规则，无需从头编写。使用 `ingredient search` 查找配料，
使用 `ingredient show` 查看其 YAML。支持的应用和示例参见
[配料库](https://github.com/huaium/cockup/blob/main/ingredients/README.md)。

## 开发

使用 Cargo 构建和测试，或通过 Just 配方运行样例备份与恢复流程。
入门步骤参见[开发指南](docs/zh-cn/development.md)。

## 许可证

[MIT](LICENSE)。
