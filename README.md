# cockup

Yet another backup tool for various configurations.

## Installation

### Install from source

1. Clone or download this repository
2. Navigate to the project root, and run:

```bash
pip install -e .
```

## Usage

```bash
# List potential config paths for installed Homebrew casks
cockup list

# Backup files according to configuration
cockup backup /path/to/config.yaml

# Restore files from backup
cockup restore /path/to/config.yaml
```

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
  on_start: # Optional rule-level hooks
    - name: "Before Rule"
      command: ["echo", "Processing rule"]
  on_end:
    - name: "After Rule"
      command: ["echo", "Rule complete"]
```

### Hook Structure

Hooks support custom commands:

```yaml
- name: "Hook Name" # Required: Hook identifier
  command: ["cmd", "arg1"] # Required: Command array
  output: false # Optional: Print output (default: false)
  timeout: 10 # Optional: Timeout in seconds (default: 10)
```

Refer to [sample](sample) to view a configuration demo.

## Development

Basically, this project use `just` to unify the development workflow. If you are not familiar with it, please refer to justfile to get access to the original commands.

### Install test dependencies

This project uses `pytest` as the test framework.

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
# Test cockup backup
just sample-backup

# Or test cockup restore
just sample-restore
```

### Run test

`just test` works as an alias for it.

```bash
# Run all tests
just test

# Run with coverage
just test --cov=cockup

# Run specific test
just test tests/test_config.py -v
```
