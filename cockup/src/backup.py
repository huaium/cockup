import os
import shutil
from typing import Literal

from cockup.src.config import Config, GlobalHooks
from cockup.src.console import rprint_point
from cockup.src.hooks import run_hooks
from cockup.src.rules import handle_rules


def _handle_hooks(hooks: GlobalHooks | None, stage: Literal["pre", "post"]):
    """
    Handle global hooks for the specified stage.
    """
    if not hooks:
        return

    if stage == "pre":
        if hooks.pre_backup:
            rprint_point("Running pre-backup hooks...")
            run_hooks(hooks.pre_backup)
    elif stage == "post":
        if hooks.post_backup:
            rprint_point("Running post-backup hooks...")
            run_hooks(hooks.post_backup)


def backup(cfg: Config):
    """Perform backup operations using specified YAML configuration file."""
    rprint_point("Starting backup...")

    # Execute pre-backup hooks
    _handle_hooks(cfg.hooks, "pre")

    # Check if backup folder exists
    if cfg.clean:
        rprint_point("Clean mode enabled, will remove backup folder first if exists.")
        if cfg.destination.exists():
            rprint_point("Found existing backup folder, removing...")
            shutil.rmtree(cfg.destination)
        else:
            rprint_point("Existing backup folder not found, creating a new one.")
    else:
        rprint_point(
            "Clean mode disabled, will not remove existing backup folder, just update."
        )
    cfg.destination.mkdir(parents=True, exist_ok=True)

    # Change cwd
    # Note that before we change cwd, the path in cfg has already converted to absolute path
    os.chdir(cfg.destination)

    # Logic of notification for metadata is in `rules.py`
    # since it's where the copy behavior will be determined

    # Backup configs
    handle_rules(cfg.rules, cfg.metadata, "backup")

    # Execute post-backup hooks
    _handle_hooks(cfg.hooks, "post")

    rprint_point("Backup completed.")
