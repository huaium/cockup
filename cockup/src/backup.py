import os
import shutil

from cockup.src.config import Config
from cockup.src.console import rprint_point
from cockup.src.hooks import run_hooks
from cockup.src.rules import handle_rules


def backup(cfg: Config):
    """Perform backup operations using specified YAML configuration file."""
    rprint_point("Starting backup...")

    hooks = cfg.hooks

    # Execute pre-backup hooks
    if hooks.pre_backup:
        rprint_point("Running pre-backup hooks...")
        run_hooks(hooks.pre_backup)

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
    if hooks.post_backup:
        rprint_point("Running post-backup hooks...")
        run_hooks(hooks.post_backup)

    rprint_point("Backup completed.")
