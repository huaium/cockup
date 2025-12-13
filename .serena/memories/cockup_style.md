# Cockup Style and Conventions

- Python 3.13 typing: modules predominantly use modern type hints (e.g., `Path`, `list[str]`, `int | None`) and dataclasses for structured data (`Hook`, `Rule`, `GlobalHooks`, `Config`).
- Coding conventions: minimalist docstrings/comments (mostly on helper functions); functionality is factored into small helper functions or dataclass constructors; logic relies on `pathlib.Path` for path handling and `click` for CLI glue.
- Hooks/console helpers: custom console output lives in `cockup/src/console.py` (termcolor-based wrappers) while hook execution helpers are centralized under `cockup/src/hooks.py` to keep side effects contained.
- Configuration parsing: `yaml.safe_load` + dataclasses, constructs full config by recursively loading `include` files and merging hooks; warnings guard against unsafe hook execution.
- Tests: Pytest is used with strict markers defined in `pyproject.toml`, tests mirror the module decomposition (`test_config.py`, `test_rules.py`, etc.).
- Formatting/linting: rely on `ruff` targetting Python 3.12 (`ruff.toml`).
- CLI entrypoints: `python -m cockup.main` or the `cockup` console script via `cockup/main.py`. Keep CLI parameter docs in `HELP_*` constants, use `click` decorators with `@main.command` and explicit options.
