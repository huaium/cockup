import os
import subprocess

import click

from cockup.src.config import Config, Hook
from cockup.src.console import Style, rprint, rprint_error, rprint_point


def _get_hook_dict(cfg: Config) -> dict[str, Hook]:
    """
    Retrieve all hooks from the configuration and map their name to the hook object.
    """

    hook_dict = {}

    def add_to_dict(hooks: list[Hook]):
        for hook in hooks:
            hook_dict[hook.name] = hook

    # Rule-level hooks
    for rule in cfg.rules:
        add_to_dict(rule.on_start)
        add_to_dict(rule.on_end)

    # Global hooks
    if cfg.hooks:
        add_to_dict(cfg.hooks.pre_backup)
        add_to_dict(cfg.hooks.post_backup)
        add_to_dict(cfg.hooks.pre_restore)
        add_to_dict(cfg.hooks.post_restore)

    return hook_dict


def run_hooks(hooks: list[Hook]):
    """
    Execute hooks defined in the configuration.
    """

    success_count = 0
    total_commands = len(hooks)

    for i, hook in enumerate(hooks):
        rprint_point(f"Running hook ({i + 1}/{total_commands}): {hook.name}")

        env = os.environ.copy()
        if hook.env:
            env.update(hook.env)

        try:
            subprocess.run(
                hook.command,
                capture_output=not hook.output,
                text=True,
                check=True,
                timeout=hook.timeout,
                env=env,
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
    rprint_point(f"Completed {success_count}/{total_commands} {hook_str}.")


def run_hooks_with_input(cfg: Config):
    """
    List available hooks from the configuration and prompt the user to select some and run.
    """

    hook_dict = _get_hook_dict(cfg)

    all_hooks = list(hook_dict.values())

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


def run_hook(hook: Hook):
    run_hooks([hook])


def run_hook_by_name(cfg: Config, name: str):
    """
    Run a specific hook by its name.
    """

    hook_dict = _get_hook_dict(cfg)

    if name not in hook_dict:
        rprint_error(f"Hook `{name}` not found in the configuration.")
        return

    run_hook(hook_dict[name])
