# Cockup Suggested Commands

- `source .venv/bin/activate` (mandatory before running other commands).
- `pip install -e .` (or `pip install -e "[test]"` to include dev/test deps) to install/update the project in editable mode.
- `just sync` / `just sync-all` (via `uv`) to synchronize dependencies described in `pyproject.toml`.
- `just run [ARGS]` to exercise CLI entrypoints, e.g. `just run list`, `just run backup sample/config.yaml`.
- `python -m cockup.main <command> config.yaml` or the installed `cockup` script to invoke backup/restore/hook/list logically.
- `just sample-backup`, `just sample-restore`, `just sample-hook [hook_name]` for manual scenario tests using `sample/config.yaml`.
- `just test` (alias for `pytest` with strict markers) and `just test --cov=cockup` or `pytest tests/test_config.py -v` for targeted checks.
- `just build` (alias for `uv build`) to ensure packaging/entrypoints build without issues.
- `just clean` / `just clean-pycache` for removing `dist/` or cached python files before packaging.
- `ruff check .` (optional) to validate linting rules targeting Python 3.12 (`ruff.toml`).
- Git/basic shell utilities: `git status`, `git diff`, `git add`, `git commit`, `ls`, `cd`, `rg <pattern>` for searches, `pwd`, and `find`/`grep` as needed (Darwin-compatible standard Unix utilities).
