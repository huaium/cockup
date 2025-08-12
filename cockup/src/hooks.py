import subprocess
from typing import Any

from cockup.src.console import rprint_error, rprint_point


def run_hooks(hooks: list[dict[str, Any]]):
    """
    Execute hooks defined in the configuration.
    """

    success_count = 0
    total_commands = len(hooks)

    for i, hook in enumerate(hooks):
        name = hook.get("name")
        if not name:
            rprint_error(f"Hook {i + 1} missing `name`, skipping...")
            continue

        command = hook.get("command")
        if not command:
            rprint_error(f"Hook {i + 1} missing `command`, skipping...")
            continue

        output = hook.get("output", False)
        timeout = hook.get("timeout", 10)

        rprint_point(f"Running hook ({i + 1}/{total_commands}): {name}")

        try:
            subprocess.run(
                command,
                capture_output=not output,
                text=True,
                check=True,
                timeout=timeout,
            )

        except subprocess.TimeoutExpired:
            rprint_error(f"Command `{name}` timed out after {timeout} seconds.")

        except Exception as e:
            rprint_error(f"Error executing command `{name}`: {str(e)}.")

        else:
            success_count += 1

    hook_str = "hooks" if total_commands > 1 else "hook"
    rprint_point(f"Completed {success_count}/{total_commands} {hook_str} successfully.")
