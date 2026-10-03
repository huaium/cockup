# cockup

[![CI](https://github.com/huaium/cockup/actions/workflows/rust.yml/badge.svg)](https://github.com/huaium/cockup/actions/workflows/rust.yml)

[English](README.md) | 中文

又一个用于备份各种配置文件的工具。使用 Rust 编写，支持 Apple Silicon 和 Intel macOS。

## 安装

发布到 crates.io 后可运行：

```sh
cargo install cockup --locked
```

也可从 [GitHub Releases](https://github.com/huaium/cockup/releases) 下载对应架构的二进制文件，
校验 SHA256 后将 `cockup` 放入 `PATH`。源码构建参见[开发指南](docs/zh-cn/development.md)。

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

## 文档

- [命令参考](docs/zh-cn/commands.md)：备份、恢复、差异、验证、Hooks、发现配置、配料及缓存管理。
- [配置参考](docs/zh-cn/configuration.md)：规则、符号链接、元数据、Hooks 和配置导入。
- [开发指南](docs/zh-cn/development.md)：构建、检查及样例工作流程。
- [配料库](https://github.com/huaium/cockup/blob/main/ingredients/README.md)：预定义的应用配置。

## 许可证

[MIT](LICENSE)。
