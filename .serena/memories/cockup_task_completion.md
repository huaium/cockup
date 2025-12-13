# Cockup Task Completion Checklist

1. Activate the virtualenv (`source .venv/bin/activate`) before running any commands.
2. Install/update dependencies when necessary: `pip install -e .` (or `pip install -e "[test]"` for tests). `just sync` / `just sync-all` (uses `uv`) keep deps aligned.
3. Run automated checks: `just test` (alias for `pytest` with strict settings) and `just build` (alias for `uv build`) to ensure packaging/entrypoints still work. Optionally run `ruff check .` to cover linting.
4. Exercise CLI flows manually if relevant (e.g., `just sample-backup`, `just sample-restore`, `just sample-hook`, or `python -m cockup.main backup config.yaml`).
5. Clean up artifacts if needed: `just clean` removes `dist/`, `just clean-pycache` clears `__pycache__`.
6. Run `git status` to verify changes are intentional before sharing results.
