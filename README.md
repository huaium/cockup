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

### `cockup template`

Print a valid configuration with commented examples of every YAML field, including
optional fields and their defaults:

```sh
cockup template > config.yaml
```

### `cockup completions`

Generate a completion script for `bash`, `zsh`, `fish`, `powershell`, or `elvish`
and save it in your shell's completion directory. For example:

```sh
cockup completions zsh > _cockup
```

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

Use `--dry-run` with `backup` or `restore` to preview copies, replacements,
symlink restoration, and clean-mode removals. It reads the configuration and
backup but does not write files, change the manifest, or run hooks. Symlink and
cross-user path choices may still be requested so the preview can show their
destinations.

```sh
cockup backup /path/to/config.yaml --dry-run
cockup restore /path/to/config.yaml --dry-run
```

Use `verify` before restore to check that the backup is complete, its symlink
manifest is valid, recorded links have the expected type, and paths selected by
the current configuration are present and readable. Verification is read-only
and does not run hooks:

```sh
cockup verify /path/to/config.yaml
```

The manifest does not inventory or hash ordinary files. Verification cannot
detect changed contents or a missing child inside a backed-up directory if that
child is not selected separately by a rule.

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

# Required: referece, dereference, or prompt
symlinks: referece

# List of backup rules
rules:
  - from: "/source/directory"
    targets: ["*.conf", "*.json"]
    to: "subdirectory"
```

`destination` and `symlinks` are required when running a configuration directly.
Omit them to make the YAML an include-only ingredient; only `rules` is required
in an ingredient.
The root value is the default; include objects and rules can override it:

| Mode | Behavior |
| --- | --- |
| `referece` | Copy the link itself, preserving its target meaning by adjusting relative paths, including dangling links. |
| `dereference` | Save target contents and record the link for restoration. |
| `prompt` | Ask once during backup: `r`/`referece` or `d`/`dereference`; apply the decision to all links. |

The policy applies to matched entries and symlinks encountered while recursively
copying directories. In `prompt` mode, `--quiet` only suppresses hook confirmation;
it does not answer symlink questions. Invalid answers are prompted again; closed
input reports an error. Use `referece` or `dereference` for unattended runs. Dereferencing broken
links, cycles, or overlapping source/destination paths reports an error. Link
choices are collected before cleanup or directory replacement; planning failures
abort clean/prompt backups before deleting backup data.

Backup writes `.cockup-symlinks.json` at the backup root. Keep it with the backup.
It records the backup user/home, original link locations, exact targets, resolved
targets, handling modes, target chains, and relative paths to backed-up contents.
Restore follows the recorded modes, regardless of the current YAML mode:

- `referece`: recreate the link only; leave target contents untouched.
- `dereference`: restore target contents, then recreate the original link and any
  recorded links in its target chain.
- `prompt`: reuse the choices recorded during backup; do not ask for them again.

Reference copies keep pointing to their original targets, adjusting relative link
paths for the backup location. Restore recreates the original link text. Links
pointing into the source or destination are allowed. With `clean: true`, cleanup
removes links without following them; files inside the cleaned destination are
still subject to normal cleanup.

When restoring paths belonging to a different backup user/home, Cockup asks once:
`c`/`current` maps them to the current home; `o`/`original` retains the old home.
The choice applies to file locations, restored target contents, and link targets,
including relative links that explicitly name the old user. It matches home-path
prefixes, not arbitrary username text. `--quiet` does not skip this question.
Invalid or missing input aborts before restoration writes.

The manifest is replaced atomically after a successful backup. A failed or
interrupted copy leaves `.cockup-incomplete`; restore refuses that backup until a
successful backup clears the marker. This detects incomplete backups, but does
not roll back copies already made. Missing manifests are accepted only for legacy
`referece` restores (without user remapping); malformed manifests fail before
restoration. The manifest and incomplete-marker paths are reserved.

### Optional Fields

```yaml
# Include rules and hooks from other config files
include:
  - file: "./another_config.yaml"
    wrap: imported
    symlinks: dereference
    metadata: false

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
A failed or timed-out check is reported, and subsequent hooks in that stage still run.
If any pre-backup or pre-restore hook fails, the operation stops before cleanup,
copying, rule hooks, or post-hooks.

### Include

Use `include` objects to import rules and hooks from other configuration files. Included entries run before local entries. Each object specifies exactly one of `file` or `ingredient`. `file` is resolved relative to the configuration that declares it; `ingredient` names an entry in the [ingredient library](ingredients/README.md). Optional `wrap` places all imported backup files under that path within the root destination; nested wrappers compose.

The root configuration controls `destination` and `clean`. An included file may omit `destination` and `symlinks` to become an ingredient that cannot run independently. Root `symlinks` and `metadata` are defaults: an include object can override them for its subtree, and an individual rule can override them again. Omitted values inherit from the nearest include, then the root. An included file's own top-level settings apply when it has both required standalone fields and is run directly, but its rules use the inherited settings when included. Restore uses each rule's effective `metadata` value and the symlink modes recorded by backup.

```yaml
include:
  - file: "path_to/config_one.yaml"
    wrap: config-one
    symlinks: dereference
  - file: "path_to/config_two.yaml"
    metadata: false
  - ingredient: ghostty
    wrap: ghostty
```

`wrap` must be a non-empty relative path without `..`. String include entries are no longer accepted. Rules can also set `symlinks` or `metadata` directly. Cockup downloads named ingredients from this repository on first use and caches them under `~/Library/Caches/cockup/ingredients/v1/`. It checks for updates after seven days, using the cached copy if a refresh fails; a missing ingredient is an error. Backup records the exact ingredient YAML in its manifest, so restore and verify use the backed-up rules without contacting GitHub. Dry-run can fetch an ingredient but does not write to the cache.

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

Existing configurations must add `symlinks: referece` to retain link-copying behavior.
Commands and other YAML fields remain compatible except for `include`, which now
requires objects with a `file` field. Relative paths resolve against each
configuration file's directory. Invalid configuration and include cycles fail before
execution. Hook confirmation happens once; `--quiet` or `-q` covers included hooks.

Exit codes are 0 for success or declined confirmation, 1 for configuration/copy/hook/
Homebrew failures, and 2 for CLI usage errors. Recoverable failures do not stop later
work, but the final summary reports failure. Special files and missing literal targets
are still skipped with a warning. Metadata mode preserves file permissions, timestamps,
and supported macOS file flags; ACLs, ownership, and extended attributes are not guaranteed.

See the [migration validation notes](docs/migration.md) and [samples](sample/README.md).

## License

Please refer to [LICENSE](./LICENSE).
