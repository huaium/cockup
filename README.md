# cockup

[![CI](https://github.com/huaium/cockup/actions/workflows/rust.yml/badge.svg)](https://github.com/huaium/cockup/actions/workflows/rust.yml)

English | [中文](README_zh-cn.md)

Yet another backup tool for various configurations.

## Installation

Install with [Homebrew](https://github.com/huaium/homebrew-tap):

```sh
brew install huaium/tap/cockup
```

Or install from [crates.io](https://crates.io/crates/cockup) with Cargo:

```sh
cargo install cockup --locked
```

Prebuilt binaries are also available on
[GitHub Releases](https://github.com/huaium/cockup/releases).
For source builds, see the [development guide](docs/development.md).

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

## Commands

Preview changes with `--dry-run`, compare local files with `diff`, and check backup
integrity with `verify`. Cockup also supports hooks and Homebrew configuration
discovery. See the [command reference](docs/commands.md) for details.

## Configuration

YAML rules select files and their backup paths. Configure symlink handling,
metadata preservation, hooks, and reusable includes as needed.
See the [configuration reference](docs/configuration.md) for all fields.

## Ingredients

Include predefined application rules to avoid writing them from scratch.
Use `ingredient search` to find them and `ingredient show` to inspect their YAML.
Browse the [ingredient library](https://github.com/huaium/cockup/blob/main/ingredients/README.md)
for available applications and examples.

## Development

Build and test with Cargo, or use the Just recipes for sample backup and restore
workflows. See the [development guide](docs/development.md) to get started.

## License

[MIT](LICENSE).
