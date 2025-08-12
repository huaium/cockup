from rich.console import Console
from rich.style import Style

_console = Console()


def rprint(message: str = "", style: Style | None = None, end: str = "\n"):
    """
    Print a message to the console with optional styling.
    """
    _console.print(message, style=style, end=end, highlight=False)


def rprint_point(message: str, end: str = "\n"):
    """
    Print a message indicating a process point to the console.
    """
    rprint(
        "=> ",
        style=Style(color="cyan", bold=True),
        end="",
    )
    rprint(message=message, style=Style(bold=True, color="green"), end=end)


def rprint_error(message: str, end: str = "\n"):
    """
    Print an error message to the console.
    """
    rprint(
        "=> ",
        style=Style(color="cyan", bold=True),
        end="",
    )
    rprint(message=message, style=Style(color="red", bold=True), end=end)


def rprint_warning(message: str, end: str = "\n"):
    """
    Print a warning message to the console.
    """
    rprint(
        "=> ",
        style=Style(color="cyan", bold=True),
        end="",
    )
    rprint(message=message, style=Style(color="yellow", bold=True), end=end)
