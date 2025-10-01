import os
import platform
import re
import subprocess
from concurrent.futures import ThreadPoolExecutor, as_completed
from subprocess import CalledProcessError, TimeoutExpired

from cockup.src.console import rprint_error

env = os.environ.copy()
env["HOMEBREW_NO_AUTO_UPDATE"] = "1"


def _is_brew_installed() -> bool:
    """
    Check if Homebrew is installed and accessible.
    """
    try:
        subprocess.run(
            ["brew", "--version"],
            capture_output=True,
            text=True,
            check=True,
            timeout=5,
            env=env,
        )
        return True
    except (
        CalledProcessError,
        TimeoutExpired,
    ):
        return False
    except Exception as e:
        rprint_error(f"Error checking Homebrew installation: {e}")
        return False


def _process_cask(cask) -> tuple[str, list[str]]:
    """
    Process a single cask and return its zap items if any.
    """

    try:
        # Get cask formula using brew cat
        cat_result = subprocess.run(
            ["brew", "cat", "--cask", cask],
            capture_output=True,
            text=True,
            check=True,
            timeout=5,
            env=env,
        )

        content = cat_result.stdout

        # Find all quoted paths in the zap section
        zap_items = []
        lines = content.split("\n")
        in_zap = False
        in_trash_or_rmdir = False

        for line in lines:
            line = line.strip()

            # Check for start of zap section
            if not in_zap and line.startswith("zap"):
                in_zap = True

            if not in_trash_or_rmdir:
                directive_match = re.search(r"(trash|rmdir):", line)
                in_trash_or_rmdir = True if directive_match else False

            # Check for end of zap section
            if in_zap and line.startswith("end"):
                in_zap = False

            # If we're in the zap section, look for paths
            if in_zap and in_trash_or_rmdir:
                # Find quoted strings (paths)
                for path_match in re.finditer(r'"([^"]+)"|\'([^\']+)\'', line):
                    # Match content wrapped in quotation marks, whether single or double
                    path = path_match.group(1) or path_match.group(2)
                    # Replace version placeholders with *
                    path = re.sub(r"#\{version[^}]*\}", "*", path)
                    zap_items.append(path)

        return cask, zap_items

    except Exception as _:
        rprint_error(
            f"Error processing cask `{cask}`. Please check if it is installed."
        )
        return cask, []


def get_zap_dict(casks: list[str] = []) -> dict[str, list[str]]:
    """
    Extract zap information from all installed Homebrew casks using multiple threads.

    Returns:
        A dictionary where keys are cask names and values are lists of
        paths that would be removed by the zap stanza.
    """

    # Skip if Homebrew is not installed or on Windows
    if platform.system() == "Windows":
        return {}

    # Check if Homebrew is installed
    if not _is_brew_installed():
        rprint_error("Homebrew is not installed or not accessible.")
        return {}

    # Get list of all installed casks
    try:
        if not casks:
            # If no casks provided, fetch all installed casks
            result = subprocess.run(
                ["brew", "list", "--casks"],
                capture_output=True,
                text=True,
                check=True,
                timeout=5,
                env=env,
            )
            stripped_result = result.stdout.strip()
            casks = stripped_result.split("\n") if stripped_result else []

    except Exception as e:
        rprint_error(f"Error fetching installed casks: {e}")
        return {}

    if not casks:
        return {}

    zap_dict = {}

    # Use ThreadPoolExecutor to process casks in parallel
    with ThreadPoolExecutor(max_workers=min(16, len(casks))) as executor:
        # Submit all casks for processing
        future_to_cask = {executor.submit(_process_cask, cask): cask for cask in casks}

        # Collect results as they complete
        for future in as_completed(future_to_cask):
            cask, zap_items = future.result()
            if zap_items:
                zap_dict[cask] = zap_items

    return zap_dict
