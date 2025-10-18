import platform
from typing import Optional

import click

from cockup import __version__
from cockup.src.backup import backup
from cockup.src.config import read_config
from cockup.src.console import rprint, rprint_error, rprint_point
from cockup.src.hooks import run_hook_by_name, run_hooks_with_input
from cockup.src.list import get_zap_dict
from cockup.src.restore import restore

SHORT_HELP = "Yet another backup tool for various configurations."
SHORT_HELP_LIST = "List potential configs of installed Homebrew casks."
SHORT_HELP_RESTORE = "Restore configurations from backup."
SHORT_HELP_BACKUP = "Perform backup operations using specified YAML configuration file."
SHORT_HELP_HOOK = "Run hooks defined in the configuration."

HELP_LIST = f"""
{SHORT_HELP_LIST}

Example: 

- cockup list

- cockup list cask_name
"""
HELP_BACKUP = f"""
{SHORT_HELP_BACKUP}

Example: cockup backup config.yaml
"""
HELP_RESTORE = f"""
{SHORT_HELP_RESTORE}

Example: cockup restore config.yaml
"""
HELP_HOOK = f"""
{SHORT_HELP_HOOK}

Examples:

- cockup hook config.yaml   # Interactive mode

- cockup hook config.yaml --name hook_name  # Run specific hook
"""


@click.group(
    context_settings=dict(help_option_names=["-h", "--help"]),
    invoke_without_command=True,
)
@click.version_option(
    version=__version__, prog_name="cockup", message="%(prog)s v%(version)s"
)
@click.pass_context
def main(ctx):
    f"""
    {SHORT_HELP}
    """

    if ctx.invoked_subcommand is None:
        click.echo(ctx.get_help())
        ctx.exit(0)


@main.command(
    "list",
    short_help=SHORT_HELP_LIST,
    help=HELP_LIST,
)
@click.argument("casks", nargs=-1, type=click.STRING)
def list_command(casks):
    # Skip on Windows as Homebrew is not available
    if platform.system() == "Windows":
        rprint_error("This command is not supported on Windows.")
        return

    rprint_point("Retrieving potential configs from Homebrew...")
    zap_dict = get_zap_dict(list(casks))

    if not zap_dict:
        rprint_point("No potential configs found.")
        return

    for package, items in zap_dict.items():
        rprint()  # Print a newline for better readability
        rprint_point(f"{package}:")
        for item in items:
            rprint(f"  {item}")


@main.command(
    "restore",
    short_help=SHORT_HELP_RESTORE,
    help=HELP_RESTORE,
)
@click.argument("config_file", type=click.Path(exists=True))
@click.option("--quiet", "-q", help="Suppress warning messages.", is_flag=True)
def restore_command(config_file: str, quiet: bool = False):
    cfg = read_config(config_file, quiet)

    if not cfg:
        return

    restore(cfg)


@main.command(
    "backup",
    short_help=SHORT_HELP_BACKUP,
    help=HELP_BACKUP,
)
@click.argument("config_file", type=click.Path(exists=True))
@click.option("--quiet", "-q", help="Suppress warning messages.", is_flag=True)
def backup_command(config_file: str, quiet: bool = False):
    cfg = read_config(config_file, quiet)

    if not cfg:
        return

    backup(cfg)


@main.command(
    "hook",
    short_help=SHORT_HELP_HOOK,
    help=HELP_HOOK,
)
@click.argument("config_file", type=click.Path(exists=True))
@click.option("--name", "-n", help="Name of a specific hook to run.")
@click.option("--quiet", "-q", help="Suppress warning messages.", is_flag=True)
def hook_command(config_file: str, name: Optional[str] = None, quiet: bool = False):
    cfg = read_config(config_file, quiet)

    if not cfg:
        return

    if name:
        # Run a single hook by name
        run_hook_by_name(cfg, name)
    else:
        # Run interactive hook selection
        run_hooks_with_input(cfg)


if __name__ == "__main__":
    main()
