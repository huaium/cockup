import glob
import os
import shutil
import stat
from pathlib import Path
from typing import Literal

from cockup.src.config import Rule
from cockup.src.console import Style, rprint, rprint_error, rprint_point, rprint_warning
from cockup.src.hooks import run_hooks


def _abbreviate_home(path: Path) -> str:
    """
    Abbreviate the user's home directory in the given path.
    """

    # Do not resolve, and keep as is
    # so that even the it is a symlink, the original path can still be preserved
    full_path = path.expanduser().absolute()

    try:
        rel_path = full_path.relative_to(Path.home())
        return f"~/{rel_path}"
    except Exception:
        return str(full_path)


def _smart_copy(src: Path, dst: Path, metadata: bool, print_progress: bool = True):
    def rprint_inner(message: str, style: Style | None = None, end: str = "\n"):
        if print_progress:
            rprint(message, style=style, end=end)

    def copy_inner(src: Path, dst: Path):
        # Always copy the file itself, not the target
        if metadata:
            shutil.copy2(src, dst, follow_symlinks=False)
        else:
            shutil.copy(src, dst, follow_symlinks=False)

    try:
        mode = os.lstat(src).st_mode
        if stat.S_ISREG(mode) or stat.S_ISLNK(mode):
            file_type = "Symlink" if stat.S_ISLNK(mode) else "File"
            updating = False

            if dst.exists():
                rprint_inner(
                    f"{file_type} existed, updating: ", style=Style(bold=True), end=""
                )
                rprint_inner(f"{_abbreviate_home(src)}")
                updating = True
                dst.unlink(missing_ok=True)

            # To avoid folder not found
            dst.parent.mkdir(parents=True, exist_ok=True)

            copy_inner(src, dst)

            if not updating:
                rprint_inner(f"{file_type} copied: ", style=Style(bold=True), end="")
                rprint_inner(f"{_abbreviate_home(src)}")

        elif stat.S_ISDIR(mode):
            updating = False

            if dst.exists():
                rprint_inner(
                    "Folder existed, updating: ", style=Style(bold=True), end=""
                )
                rprint_inner(f"{_abbreviate_home(dst)}")
                updating = True
                shutil.rmtree(dst)

            # Walk the folder to avoid copying special files like sockets
            dst.mkdir(parents=True, exist_ok=True)
            for entry_src in src.iterdir():
                _smart_copy(
                    src=entry_src,
                    dst=dst / entry_src.name,
                    metadata=metadata,
                    print_progress=False,
                )

            if not updating:
                rprint_inner("Folder copied: ", style=Style(bold=True), end="")
                rprint_inner(f"{_abbreviate_home(src)}")

        else:
            # Handle non-regular files (e.g., sockets, FIFOs)
            rprint_warning(
                f"Skipping non-regular file: {_abbreviate_home(src)} ({oct(mode)})"
            )

    except Exception as e:
        rprint_error(f"Error copying: {_abbreviate_home(src)}")
        rprint_error(str(e))


def _handle_rule(rule: Rule, metadata: bool, direction: Literal["backup", "restore"]):
    def smart_copy_inner(src: Path, dst: Path):
        _smart_copy(src=src, dst=dst, metadata=metadata)

    for target in rule.targets:
        if direction == "backup":
            source_path = (rule.src / target).absolute()
            dest_dir_path = (Path.cwd() / rule.to).absolute()
            glob_base = rule.src
        else:  # restore
            source_path = (Path.cwd() / rule.to / target).absolute()
            dest_dir_path = rule.src.absolute()
            glob_base = Path.cwd() / rule.to

        # Check if the path exists directly
        if source_path.exists():
            dest_path = (dest_dir_path / target).absolute()

            smart_copy_inner(source_path, dest_path)
        elif glob.has_magic(source_path.as_posix()):
            # Use glob to find all matching files
            matched_paths = [
                Path(item).absolute() for item in glob.glob(source_path.as_posix())
            ]

            if matched_paths:
                rprint_point(
                    f"Pattern matched ({len(matched_paths)} founded): {_abbreviate_home(source_path)}"
                )

                # Process each matched file
                for matched_path in matched_paths:
                    # Generate destination file path and copy the file
                    dest_path = dest_dir_path / matched_path.relative_to(glob_base)

                    smart_copy_inner(matched_path, dest_path)
            else:
                rprint_error(
                    f"Matches not found for pattern: {_abbreviate_home(source_path)}"
                )
        else:
            rprint_error(f"Source not found, skipping: {_abbreviate_home(source_path)}")


def handle_rules(
    rules: list[Rule], metadata: bool, direction: Literal["backup", "restore"]
):
    """
    Dump the files and folders following the specified rules to the target folder.
    """

    if metadata:
        rprint_point("Metadata preservation enabled.")
    else:
        rprint_point("Metadata preservation disabled.")

    # Iterate over rules list
    for index, rule in enumerate(rules):
        if rule.on_start:
            rprint_point(f"Running pre-rule hooks for Rule {index + 1}...")
            run_hooks(rule.on_start)

        _handle_rule(rule, metadata, direction)

        if rule.on_end:
            rprint_point(f"Running post-rule hooks for Rule {index + 1}...")
            run_hooks(rule.on_end)
