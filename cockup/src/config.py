from dataclasses import dataclass
from pathlib import Path
from typing import Any

import yaml

from cockup.src.console import rprint_error


@dataclass
class Rule:
    src: Path
    targets: list[str]
    to: str
    on_start: list[dict[str, Any]]
    on_end: list[dict[str, Any]]


@dataclass
class Hooks:
    pre_backup: list[dict[str, Any]]
    post_backup: list[dict[str, Any]]
    pre_restore: list[dict[str, Any]]
    post_restore: list[dict[str, Any]]


@dataclass
class Config:
    destination: Path
    rules: list[Rule]
    hooks: Hooks
    clean: bool
    metadata: bool


def _read_yaml(file_path: str) -> dict[str, Any]:
    try:
        with open(file_path, "r") as file:
            yaml_data = yaml.safe_load(file)
            return yaml_data
    except Exception as e:
        rprint_error(f"Error reading YAML file: {e}")

    return {}


def _handle_destination(config: dict[str, Any]) -> Path | None:
    if not config.get("destination"):
        rprint_error("No destination specified in config.")
        return

    # Now `destination` must exist
    destination = Path(config["destination"]).expanduser().absolute()

    if not config.get("rules"):
        rprint_error("No rules specified in config.")

        if config.get("rule"):
            rprint_error("Did you mistakenly use `rule` instead of `rules`?")

        return

    return destination


def _handle_rules(config: dict[str, Any]) -> list[Rule] | None:
    if not config.get("rules"):
        rprint_error("No rules specified in config.")

        if config.get("rule"):
            rprint_error("Did you mistakenly use `rule` instead of `rules`?")

        return

    # Now `rules` must exist
    rules = []
    for index, item in enumerate(config["rules"]):
        if not isinstance(item, dict):
            rprint_error("Each rule must be a dictionary. Please review your config.")
            return

        if "from" not in item:
            rprint_error(f"Rule {index + 1} is missing `from`.")
            return

        if "targets" not in item:
            rprint_error(f"Rule {index + 1} is missing `targets`.")
            return

        if "to" not in item:
            rprint_error(f"Rule {index + 1} is missing `to`.")
            return

        # Handle rule-specific hooks
        on_start_hooks = item.get("on-start", [])
        on_end_hooks = item.get("on-end", [])

        rules.append(
            Rule(
                src=Path(item["from"]).expanduser().absolute(),
                targets=item["targets"],
                to=item["to"],
                on_start=on_start_hooks,
                on_end=on_end_hooks,
            )
        )

    return rules


def _handle_hooks(config: dict[str, Any]) -> Hooks:
    hooks = config.get("hooks", {})  # Default to empty dict

    pre_backup_hooks = hooks.get("pre-backup", [])  # Default to empty list
    post_backup_hooks = hooks.get("post-backup", [])
    pre_restore_hooks = hooks.get("pre-restore", [])
    post_restore_hooks = hooks.get("post-restore", [])

    return Hooks(
        pre_backup=pre_backup_hooks,
        post_backup=post_backup_hooks,
        pre_restore=pre_restore_hooks,
        post_restore=post_restore_hooks,
    )


def read_config(file_path: str) -> Config | None:
    """
    Read the configuration from a YAML file.

    Returns:
        A Config object if the configuration is valid, None otherwise.
    """
    config = _read_yaml(file_path)

    if not config:
        return

    # Handle destination
    destination = _handle_destination(config)
    if not destination:
        return

    # Handle rules
    rules = _handle_rules(config)
    if not rules:
        return

    # Handle clean and metadata options
    clean: bool = config.get("clean", False)  # Default to False
    metadata: bool = config.get("metadata", True)  # Default to True

    # Handle hooks
    hooks = _handle_hooks(config)

    return Config(
        destination=destination,
        rules=rules,
        hooks=hooks,
        clean=clean,
        metadata=metadata,
    )
