from typing import Literal

from termcolor import cprint


class Style:
    def __init__(
        self,
        color: Literal[
            "black",
            "red",
            "green",
            "yellow",
            "blue",
            "magenta",
            "cyan",
            "white",
        ]
        | None = None,
        bold: bool = False,
        dark: bool = False,
        underline: bool = False,
        blink: bool = False,
        reverse: bool = False,
        concealed: bool = False,
        strike: bool = False,
    ):
        self.color = color
        self.bold = bold
        self.dark = dark
        self.underline = underline
        self.blink = blink
        self.reverse = reverse
        self.concealed = concealed
        self.strike = strike

    def to_attrs(self) -> list[str]:
        attrs = []
        if self.bold:
            attrs.append("bold")
        if self.dark:
            attrs.append("dark")
        if self.underline:
            attrs.append("underline")
        if self.blink:
            attrs.append("blink")
        if self.reverse:
            attrs.append("reverse")
        if self.concealed:
            attrs.append("concealed")
        if self.strike:
            attrs.append("strike")
        return attrs


def rprint(
    message: str = "",
    style: Style | None = None,
    end: str = "\n",
):
    """
    Print a message to the console with optional styling.
    """
    cprint(
        message,
        color=style.color if style else None,
        attrs=style.to_attrs() if style else None,
        end=end,
    )


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
