# cockup

![PyPI - Version](https://img.shields.io/pypi/v/cockup)

English | [中文](README_zh-cn.md)

Yet another backup tool for various configurations.

## Installation

### Homebrew

```bash
# Single-line installation
brew install huaium/tap/cockup

# Or, using `brew tap`
brew tap huaium/tap
brew install cockup
```

### PyPI

```bash
pip install cockup
```

### Install from source

1. Clone or download this repository
2. Navigate to the project root, and run:

```bash
pip install -e .
```

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

Run a specified hook interactively.

## Configuration

Create a YAML configuration file with the following structure:

### Required Fields

```yaml
# Where backups are stored
destination: "/path/to/backup/directory"

# List of backup rules
rules:
  - from: "/source/directory"
    targets: ["*.conf", "*.json"]
    to: "subdirectory"
```

### Optional Fields

```yaml
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
- from: "/source/directory"
  targets:
    # Folders or files under `from`
    # Wildcards are supported
    - "pattern1"
    - "pattern2"
  to: "backup/subdirectory" # A folder under `destination`
  on-start: # Optional rule-level hooks
    - name: "Before Rule"
      command: ["echo", "Processing rule"]
  on-end:
    - name: "After Rule"
      command: ["echo", "Rule complete"]
```

### Hook Structure

Hooks support custom commands.

If you want to run them within a specified shell, use commands like `bash -c` after ensuring your commands are safe.

```yaml
- name: "Hook Name" # Required: Hook identifier
  command: ["cmd", "arg1"] # Required: Command args list
  output: false # Optional: Print output (default: false)
  timeout: 10 # Optional: Timeout in seconds (default: 10)
```

For example, you may want to use it to dump Homebrew bundle into a file and place it under the folder defined by `destination`:

```yaml
- name: "Brewfile Dumping"
  command: ["brew", "bundle", "dump", "--force", "--file", "Brewfile"]
  output: true
  timeout: 10
```

Refer to [sample](sample) to view a configuration demo.

## Development

Basically, this project use `just` to unify the development workflow. If you are not going to use it, please refer to `justfile` in the project root to get access to the original commands.

### Install test dependencies

Use `pytest` as the test framework.

```bash
just install-test
```

### Run directly

With the command form of `just run [ARGS]`.

```bash
# `cockup list`
just run list

# `cockup backup`
just run backup /path/to/config.yaml
```

### Run sample

A [sample](sample) with minimal configs is provided for manual testing.

```bash
# Test `cockup backup`
just sample-backup

# Or test `cockup restore`
just sample-restore
```

### Test

`just test` works as an alias for `pytest`.

```bash
# Run all tests
just test

# Run with coverage
just test --cov=cockup

# Run specific test
just test tests/test_config.py -v
```

### Build

`just build` works as an alias for `uv build`.

## License

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
