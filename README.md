# cockup

[![CI](https://github.com/huaium/cockup/actions/workflows/rust.yml/badge.svg)](https://github.com/huaium/cockup/actions/workflows/rust.yml)

English | [中文](README_zh-cn.md)

Yet another backup tool for various configurations.

## Installation

Cockup 0.2.0 is written in Rust and supports Apple Silicon and Intel macOS.
Python is no longer required.

Once released, download the matching archive from
[GitHub Releases](https://github.com/huaium/cockup/releases), verify its SHA256,
and place the extracted `cockup` binary in a directory on `PATH`.
After publication to crates.io, you can also install it with:

```sh
cargo install cockup --locked
```

To build from source:

```sh
cargo build --release --locked
./target/release/cockup --help
```

Existing PyPI releases remain available but do not install the Rust version.
The existing Homebrew tap requires a separate formula update.

## Usage

### `cockup list`

You may want to use it as a reference when writing your own backup rules.

```bash
# List potential config paths for all installed Homebrew casks
cockup list

# List potential config paths for specified cask
cockup list cask-name-1 [cask-name-n...]
```

### `cockup backup & restore`

```bash
# Backup files according to configuration
cockup backup /path/to/config.yaml

# Restore files from backup
cockup restore /path/to/config.yaml
```

### `cockup hook`

```bash
# Run hooks interactively
cockup hook /path/to/config.yaml

# Or, run a specified hook by its name
cockup hook /path/to/config.yaml --name hook_name
```

## Configuration

Create a YAML configuration file with the following structure:

### Required Fields

```yaml
# Where backups are stored
# If you use relative path, it will be relative to the config file's directory
destination: "/path/to/backup/directory"

# List of backup rules
rules:
  - from: "/source/directory"
    targets: ["*.conf", "*.json"]
    to: "subdirectory"
```

### Optional Fields

```yaml
# Include rules and hooks from other config files
include:
  - "./another_config.yaml"

# Clean mode, whether to remove existing backup folder (default: false)
clean: false

# Whether to preserve metadata when backing up (default: true)
metadata: true

# Global hooks
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

### Rule Structure

Each rule defines what to backup:

```yaml
- from: "/source/directory/in/pattern" # Wildcards are supported
  targets:
    # Folders or files under `from`
    - "pattern1" # Wildcards are also supported
    - "relative/path/to/file"
  to: "backup/subdirectory" # A folder under `destination`
  on-start: # Optional rule-level hooks
    - name: "Before Rule"
      command: ["echo", "Processing rule"]
  on-end:
    - name: "After Rule"
      command: ["echo", "Rule complete"]
```

Please be aware that when using wildcards in `from`, to prevent duplicate files caused by matching multiple directories, cockup will locate the first safe directory without wildcards and set it as `from`. The paths containing wildcards encountered during the search process will be prepended to each `target`.

For example, if you have:

```yaml
- from: "/source/directory/in/prefix*/pattern"
  targets:
    - "relative/path/to/file"
  to: "backup/subdirectory"
```

We will finally get:

```yaml
- from: "/source/directory/in"
  targets:
    - "prefix*/pattern/relative/path/to/file"
  to: "backup/subdirectory"
```

### Hook Structure

Hooks support custom commands.

By default, you will be prompted to confirm if your configuration file contains any hooks. Use the flag `--quiet` or `-q` to suppress it.

If you want to run them within a specified shell, use commands like `bash -c` after ensuring your commands are safe.

```yaml
- name: "Hook Name" # Required: Hook identifier
  command: ["cmd", "arg1"] # Required: Command args list
  output: false # Optional: Print output (default: false)
  timeout: 10 # Optional: Timeout in seconds
  env: # Optional: environment variables used for the command
    ENV_1: 1
    ENV_2: 2
```

Please note that you cannot pass environment variables directly using syntax like `$ENV_1`, but the subprocess you launch can access those variables.

For example, you may want to use it to dump Homebrew bundle into a file and place it under the folder defined by `destination`:

```yaml
- name: "Brewfile Dumping"
  command: ["brew", "bundle", "dump", "--force", "--file", "Brewfile"]
  check: ["test", "-s", "Brewfile"]
  output: true
  timeout: 10
```

An optional `check` command runs after `command` exits successfully. Both must exit
with code 0 for the hook to count as successful. The check uses the same working
directory, `env`, and `output`; `timeout` applies separately to each command.
A failed or timed-out check is reported, and subsequent hooks still run.

### Include

You can simply include rules and hooks from other configuration files using `include`.

Please note that only rules and hooks will be included, and they will be placed before the rules and hooks defined in the main config, using the `clean` and `metadata` fields from the main config for execution.

```yaml
include:
  - "path_to/config_one.yaml"
  - "path_to/config_two.yaml"
```

Refer to [sample](sample) to view a configuration demo.

## Development

Use native macOS and a Rust toolchain supporting Rust 2024. No virtual environment is needed.

```sh
cargo run --locked -- list
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
```

`just` lists recipes; `just run`, `just test`, `just build`, and sample recipes wrap Cargo.
Tests exercise the CLI using temporary files and executable Homebrew/hook fixtures.
For a real Homebrew smoke test, run `cargo run --locked -- list iterm2`.

## Migrating from Python

Commands and YAML fields remain compatible. Relative paths resolve against each
configuration file's directory. Invalid configuration and include cycles fail before
execution. Hook confirmation happens once; `--quiet` (or `--yes`) covers included hooks.

Exit codes are 0 for success or declined confirmation, 1 for configuration/copy/hook/
Homebrew failures, and 2 for CLI usage errors. Recoverable failures do not stop later
work, but the final summary reports failure. Special files and missing literal targets
are still skipped with a warning. Metadata mode preserves file permissions, timestamps,
and supported macOS file flags; ACLs, ownership, and extended attributes are not guaranteed.

See the [migration validation notes](docs/migration.md) and [samples](sample/README.md).

## License

Please refer to [LICENSE](./LICENSE).
