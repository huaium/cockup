# Cockup Overview

- Purpose: Cockup is a rule-based configuration backup and restore tool that copies user-specified files and directories between source locations and a backup destination, preserves metadata, and runs hooks for custom operations.
- Tech stack: Python 3.13 with `click` for CLI, `PyYAML` for config parsing, `termcolor` for console output, `uv` for build/sync commands, and `pytest`/`pytest-cov`/`pytest-mock` for tests; packaging is setuptools-based with a `cockup` console script.
- Architecture: `cockup/main.py` exposes Click commands (backup/restore/hook/list), which rely on `cockup/src/config.py` (dataclasses + path expansion + hook detection), `cockup/src/backup.py`, `cockup/src/restore.py`, `cockup/src/rules.py`, `cockup/src/hooks.py`, `cockup/src/console.py`, and `cockup/src/list.py` for Homebrew discovery.
- Configuration: YAML files define `destination`, `rules` (with `from`/`targets`/`to`), optional `include`, `clean`, `metadata`, and hooks (`pre/post` at global level and `on-start`/`on-end` per rule). 
- Testing/files: Tests cover rules, hooks, CLI, config parsing, backup/restore engines, console output, and Homebrew integration under `tests/` and rely on pytest markers defined in `pyproject.toml`.
- Sample assets: `sample/config.yaml` plus `sample/src` and `sample/dst` for manual backup/restore/hook `just` commands.
- Codestyle cues: flat module layout, dataclasses with explicit typing, minimal docstrings, use of `typing.Optional` and modern union syntax, prefer helper functions for CLI output via `cockup/src/console.py` with termcolor output.
