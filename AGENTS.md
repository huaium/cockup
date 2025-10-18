# AGENTS.md

This file provides guidance to AI coding agents (such as Claude Code, Cursor, etc.) when working with code in this repository.

## Project Overview

Cockup is a configuration backup and restore tool that copies files and folders between user-specified locations and a backup directory. It supports pattern matching, metadata preservation, and hooks for custom operations. The tool provides backup, restore, hook, and list commands with a flexible rule-based configuration system.

## Important: Virtual Environment

**Before executing any commands, you MUST activate the `.venv` virtual environment:**

```bash
source .venv/bin/activate
```

This ensures all commands run with the correct Python dependencies and environment configuration.

## Development Commands

### Installation & Setup

```bash
# Install in development mode
pip install -e .

# Install with test dependencies
pip install -e ".[test]"
```

### Running the Tool

```bash
# Quick run with justfile
just run

# Backup command
cockup backup /path/to/config.yaml

# Restore command
cockup restore /path/to/config.yaml

# List potential Homebrew cask configs
cockup list

# Run hooks from configuration
cockup hook /path/to/config.yaml

# Run specific hook by name
cockup hook /path/to/config.yaml --name hook_name

# Direct execution (development)
python -m cockup.main backup config.yaml
python -m cockup.main restore config.yaml
```

### Testing

```bash
# Run all tests
pytest

# Run with coverage
pytest --cov=cockup

# Run specific test file
pytest tests/test_config.py -v

# Run single test
pytest tests/test_config.py::TestReadConfig::test_read_valid_config -v
```

## Architecture Overview

### Core Components

**Main Entry Point** (`cockup/main.py`):

- Click-based CLI with backup, restore, hook, and list subcommands
- Handles configuration loading and delegates to appropriate operation modules
- Hook command supports interactive selection or specific hook execution by name
- Imports from `cockup.src.*` modules

**Configuration System** (`cockup/src/config.py`):

- Uses Pydantic for robust data validation and type safety
- `Config` model holds: destination, rules (list of Rule objects), hooks, clean mode, metadata flag
- `Rule` model with: src (source path), targets (pattern list), to (destination), on_start/on_end hooks
- `Hook` model with: name, command, output flag, timeout
- `GlobalHooks` model with: pre_backup, post_backup, pre_restore, post_restore hook lists
- `read_config()` parses YAML with comprehensive validation and error reporting
- Expands `~` paths and converts to absolute paths automatically
- Provides security warnings when hooks are detected in configuration

**Backup Engine** (`cockup/src/backup.py`):

- `backup()` orchestrates the full backup workflow
- Executes pre-backup hooks, handles clean mode, changes to destination directory
- Calls `handle_rules()` from rules.py to process file operations
- Executes post-backup hooks and provides completion messaging

**Restore Engine** (`cockup/src/restore.py`):

- `restore()` orchestrates the full restore workflow
- Executes pre-restore hooks, changes to destination directory
- Calls `handle_rules()` from rules.py to process file operations in reverse
- Executes post-restore hooks and provides completion messaging

**Rules Processing** (`cockup/src/rules.py`):

- `handle_rules()` processes Rule objects for both backup and restore operations
- Handles pattern matching for targets within source directories
- Preserves directory structure and handles metadata preservation
- Supports both backup (source→destination) and restore (destination→source) modes

**Hooks System** (`cockup/src/hooks.py`):

- `run_hooks()` executes lists of hook commands sequentially
- `run_hooks_with_input()` provides interactive hook selection interface
- `run_hook_by_name()` executes a specific hook by name
- `_get_hook_dict()` builds comprehensive hook registry from configuration
- Each hook has: name (required), command array, output flag, timeout
- Supports timeout and output capture configuration with error handling
- Progress reporting with success/failure counts

**Console Output** (`cockup/src/console.py`):

- Rich-based colored output with consistent formatting
- `rprint_point()` for process steps (cyan arrow + green text)
- `rprint_error()` for errors (cyan arrow + red text)
- `rprint_warning()` for warnings (cyan arrow + yellow text)

**Homebrew Integration** (`cockup/src/zap.py`):

- `get_zap_dict()` extracts cleanup paths from Homebrew cask formulas
- Used by the `list` command to show potential configuration paths
- Not integrated into main backup/restore workflow but provides discovery functionality

### Key Design Patterns

**Pydantic Validation**: Configuration uses Pydantic models for type safety, validation, and automatic path expansion with detailed error reporting.

**Hook Management**: Comprehensive hook system with interactive selection, name-based execution, and centralized hook registry across global and rule-level hooks.

**Security Awareness**: Configuration parser warns users about hook presence and requires confirmation for enhanced security.

**Clean Mode Logic**: When enabled, removes entire backup directory before starting. When disabled, updates files in-place with "updating" vs "copied" messages.

**Bidirectional Operations**: Same rules engine handles both backup (source→destination) and restore (destination→source) by switching operation mode.

**Hook Lifecycle**: Supports pre/post hooks at both global level (backup/restore operations) and rule level (individual file operations).

## Configuration Format

The tool uses YAML configuration with these sections:

### Required Fields

- `destination`: Backup target directory (required)
- `rules`: List of rule objects defining what to backup/restore (required)

### Optional Fields

- `clean`: Whether to remove existing backup first (default: false)
- `metadata`: Whether to preserve file timestamps/permissions (default: true)
- `hooks`: Global hooks for backup/restore operations

### Rule Structure

Each rule in the `rules` list must have:

- `from`: Source directory path (mapped to `src` field in Pydantic model)
- `targets`: List of file patterns to match within the source
- `to`: Destination subdirectory within backup location
- `on-start`: Optional hook list to run before processing this rule (mapped to `on_start`)
- `on-end`: Optional hook list to run after processing this rule (mapped to `on_end`)

### Hook Structure

Hooks can be defined globally or per-rule:

- `name`: Required hook name for identification
- `command`: Required command array to execute
- `output`: Optional boolean to capture output (default: false)
- `timeout`: Optional timeout in seconds (default: 10)

Global hooks include:

- `pre-backup`: Run before backup operations (mapped to `pre_backup`)
- `post-backup`: Run after backup operations (mapped to `post_backup`)
- `pre-restore`: Run before restore operations (mapped to `pre_restore`)
- `post-restore`: Run after restore operations (mapped to `post_restore`)

## Testing Strategy

The test suite uses pytest with fixtures for temporary files/configs. Key test categories:

- Configuration parsing and validation (`test_config.py`)
- Main workflow and CLI commands (`test_main.py`)
- Backup operations and workflows (`test_backup.py`)
- Restore operations and workflows (`test_restore.py`)
- Hook execution and error handling (`test_hooks.py`)
- Rules processing and file operations (`test_rules.py`)
- Console output formatting (`test_console.py`)
- Homebrew integration functionality (`test_zap.py`)

## Special Features

**Bidirectional Operations**: Single configuration supports both backup and restore operations using the same rule definitions.

**Hook Management**: Standalone hook command provides both interactive selection and direct execution capabilities for debugging and testing hook configurations.

**Pydantic Integration**: Modern configuration system with automatic validation, type conversion, and field mapping (e.g., `from`→`src`, `on-start`→`on_start`).

**Enhanced Security**: Configuration loading includes hook detection warnings and user confirmation prompts to prevent accidental execution of potentially dangerous commands.

**Pattern Matching**: Supports file patterns within source directories for selective backup/restore.

**Metadata Preservation**: Uses shutil.copy2 when metadata=true to preserve timestamps and permissions.

**Progress Reporting**: Distinguishes between "copied" (new) and "updated" (overwritten) files for better user feedback.

**Homebrew Discovery**: List command shows potential configuration paths from installed Homebrew casks for easy setup.
