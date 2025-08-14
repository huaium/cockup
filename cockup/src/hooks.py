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
    for rule in cfg.rules:
        all_hooks.extend(rule.on_start)
        all_hooks.extend(rule.on_end)

    # Global hooks
    if not cfg.hooks:
        return

    all_hooks.extend(cfg.hooks.pre_backup)
    all_hooks.extend(cfg.hooks.post_backup)
    all_hooks.extend(cfg.hooks.pre_restore)
    all_hooks.extend(cfg.hooks.post_restore)

    return all_hooks


def select_and_run_hooks(cfg: Config):
    """
    List available hooks from the configuration and prompt the user to select some and run.
    """

    all_hooks = _get_all_hooks(cfg)

    if not all_hooks:
        rprint_error("No hooks defined in the configuration.")
        return

    rprint_point("Available hooks:")
    for i, hook in enumerate(all_hooks, start=1):
        rprint(f"[{i}] ", style=Style(bold=True), end="")
        rprint(f"{hook.name}")

    try:
        choices = click.prompt("Select hooks (separate by comma)", type=str)
        hook_ids = [
            int(choice.strip()) for choice in choices.split(",") if choice.strip()
        ]
        if hook_ids:
            run_hooks([all_hooks[i - 1] for i in hook_ids if 1 <= i <= len(all_hooks)])
    except Exception as e:
        rprint_error(f"Input invalid: {e}")
        return
