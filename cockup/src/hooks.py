import subprocess

import click

from cockup.src.config import Config, Hook
from cockup.src.console import Style, rprint, rprint_error, rprint_point


def run_hooks(hooks: list[Hook]):
    """
    Execute hooks defined in the configuration.
    """

    success_count = 0
    total_commands = len(hooks)

    for i, hook in enumerate(hooks):
        if not hook.name:
            rprint_error(f"Hook {i + 1} missing `name`, skipping...")
            continue

        if not hook.command:
            rprint_error(f"Hook {i + 1} missing `command`, skipping...")
            continue

        rprint_point(f"Running hook ({i + 1}/{total_commands}): {hook.name}")

        try:
            subprocess.run(
                hook.command,
                capture_output=not hook.output,
                text=True,
                check=True,
                timeout=hook.timeout,
            )

        except subprocess.TimeoutExpired:
            rprint_error(
                f"Command `{hook.name}` timed out after {hook.timeout} seconds."
            )

        except Exception as e:
            rprint_error(f"Error executing command `{hook.name}`: {str(e)}.")

        else:
            success_count += 1

    hook_str = "hooks" if total_commands > 1 else "hook"
    rprint_point(f"Completed {success_count}/{total_commands} {hook_str} successfully.")


def _get_all_hooks(cfg: Config):
    """
    Retrieve all hooks from the configuration.
    """

    all_hooks: list[Hook] = []

    # Rule-level hooks
    rules = cfg.rules
    for rule in rules:
        all_hooks.extend(rule.on_start)
        all_hooks.extend(rule.on_end)

    # Global hooks
    hooks = cfg.hooks

    if not hooks:
        return

    pre_backup_hooks = hooks.pre_backup
    post_backup_hooks = hooks.post_backup
    pre_restore_hooks = hooks.pre_restore
    post_restore_hooks = hooks.post_restore

    all_hooks.extend(pre_backup_hooks)
    all_hooks.extend(post_backup_hooks)
    all_hooks.extend(pre_restore_hooks)
    all_hooks.extend(post_restore_hooks)

    return all_hooks


def select_and_run_hook(cfg: Config):
    """
    List available hooks from the configuration and prompt the user to select one and run.
    """

    all_hooks = _get_all_hooks(cfg)

    if not all_hooks:
        rprint_error("No hooks defined in the configuration.")
        return

    rprint_point("Available hooks:")
    for i, hook in enumerate(all_hooks, start=1):
        rprint(f"[{i}] ", style=Style(bold=True), end="")
        rprint(f"{hook.name}")

    choice = click.prompt("Select a hook", type=int)
    if 1 <= choice <= len(all_hooks):
        run_hooks([all_hooks[choice - 1]])
