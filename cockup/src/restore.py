import os

from cockup.src.config import Config
from cockup.src.console import rprint_point
from cockup.src.hooks import run_hooks
from cockup.src.rules import handle_rules


def restore(cfg: Config):
    """Perform restore operations using specified YAML configuration file."""
    rprint_point("Starting restore...")

    hooks = cfg.hooks

    # Execute pre-restore hooks
    if hooks.pre_restore:
        rprint_point("Running pre-restore hooks...")
        run_hooks(hooks.pre_restore)

    # Change cwd
    # Note that before we change cwd, the path in cfg has already converted to absolute path
    os.chdir(cfg.destination)

    # Logic of notification for metadata is in `rules.py`
    # since it's where the copy behavior will be determined

    # Restore configs
    handle_rules(cfg.rules, cfg.metadata, "restore")

    # Execute post-restore hooks
    if hooks.post_restore:
        rprint_point("Running post-restore hooks...")
        run_hooks(hooks.post_restore)

    rprint_point("Restore completed.")
