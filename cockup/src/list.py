import json
import os
import platform
import subprocess
from concurrent.futures import ThreadPoolExecutor, as_completed
from subprocess import CalledProcessError

from cockup.src.console import rprint_error

_env = os.environ.copy()
_env["HOMEBREW_NO_AUTO_UPDATE"] = "1"

_cpu_count_tmp = os.cpu_count()
try:
    _cpu_count = int(_cpu_count_tmp) if _cpu_count_tmp is not None else 1
except (TypeError, ValueError):
    _cpu_count = 1
_cpu_count = max(1, min(_cpu_count, 32))


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
            env=_env,
        )
        return True
    except CalledProcessError:
        return False
    except Exception as e:
        rprint_error(f"Error checking Homebrew installation: {e}")
        return False


def _process_cask(cask) -> tuple[str, list[str]]:
    """
    Process a single cask and return its zap items if any.
    """

    try:
        # Get cask formula using brew info
        cat_result = subprocess.run(
            ["brew", "info", "--json=v2", "--cask", cask],
            capture_output=True,
            text=True,
            check=True,
            env=_env,
        )

        json_content = cat_result.stdout
        json_data = json.loads(json_content)

        cask_lst = json_data.get("casks", [])
        if not cask_lst:
            rprint_error(f"Cask `{cask}` not found in Homebrew repo.")
            return cask, []

        artifact_lst = cask_lst[0].get("artifacts", [])
        if not artifact_lst:
            rprint_error(f"Artifact for cask `{cask}` not found in Homebrew repo.")
            return cask, []

        zap_lst = []
        for artifact in artifact_lst:
            if "zap" in artifact:
                for zap_item in artifact["zap"]:
                    rmdir = zap_item.get("rmdir", [])
                    if isinstance(rmdir, str):
                        zap_lst.append(rmdir)
                    elif isinstance(rmdir, list):
                        zap_lst.extend(rmdir)

                    trash = zap_item.get("trash", [])
                    if isinstance(trash, str):
                        zap_lst.append(trash)
                    elif isinstance(trash, list):
                        zap_lst.extend(trash)

        return cask, zap_lst

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
                env=_env,
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
    with ThreadPoolExecutor(max_workers=min(_cpu_count, len(casks))) as executor:
        # Submit all casks for processing
        future_to_cask = {executor.submit(_process_cask, cask): cask for cask in casks}

        # Collect results as they complete
        for future in as_completed(future_to_cask):
            cask, zap_items = future.result()
            if zap_items:
                zap_dict[cask] = zap_items

    return zap_dict
