import os
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
        if hooks.pre_restore:
            rprint_point("Running pre-restore hooks...")
            run_hooks(hooks.pre_restore)
    elif stage == "post":
        if hooks.post_restore:
            rprint_point("Running post-restore hooks...")
            run_hooks(hooks.post_restore)


def restore(cfg: Config):
    """Perform restore operations using specified YAML configuration file."""
    rprint_point("Starting restore...")

    # Execute pre-restore hooks
    _handle_hooks(cfg.hooks, "pre")

    # Change cwd
    # Note that before we change cwd, the path in cfg has already converted to absolute path
    os.chdir(cfg.destination)

    # Logic of notification for metadata is in `rules.py`
    # since it's where the copy behavior will be determined

    # Restore configs
    handle_rules(cfg.rules, cfg.metadata, "restore")

    # Execute post-restore hooks
    _handle_hooks(cfg.hooks, "post")

    rprint_point("Restore completed.")
