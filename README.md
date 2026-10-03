# cockup

[![CI](https://github.com/huaium/cockup/actions/workflows/rust.yml/badge.svg)](https://github.com/huaium/cockup/actions/workflows/rust.yml)

English | [中文](README_zh-cn.md)

Yet another backup tool for various configurations. Written in Rust for Apple
Silicon and Intel macOS.

## Installation

Once published to crates.io:

```sh
cargo install cockup --locked
```

You can also download a matching binary from
[GitHub Releases](https://github.com/huaium/cockup/releases), verify its SHA256,
and place `cockup` on your `PATH`. See [development](docs/development.md) to build
from source.

## Typical example

Save this as `config.yaml` to back up your shell and Git configuration into a
`backup` directory beside the YAML file:

```yaml
destination: ./backup
symlinks: reference
rules:
  - from: "~"
    targets: [".zshrc", ".gitconfig"]
    to: dotfiles
```

Preview the changes, create the backup, and restore it when needed:

```sh
cockup backup config.yaml --dry-run
cockup backup config.yaml
cockup restore config.yaml
```

Restore shows differences and asks before replacing conflicting local files.

## Documentation

- [Command reference](docs/commands.md): backup, restore, diff, verify, hooks, discovery, ingredients, and cache management.
- [Configuration reference](docs/configuration.md): rules, symlinks, metadata, hooks, and includes.
- [Development](docs/development.md): builds, checks, and sample workflows.
- [Ingredient library](https://github.com/huaium/cockup/blob/main/ingredients/README.md): predefined application configurations.

## License

[MIT](LICENSE).
