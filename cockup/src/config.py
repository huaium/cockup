import os
from dataclasses import dataclass, field
from pathlib import Path

import click
import yaml

from cockup.src.console import rprint_error, rprint_warning


@dataclass
class Hook:
    name: str
    command: list[str]
    output: bool = False
    timeout: int | None = None
    env: dict[str, str] | None = None

    @classmethod
    def from_dict(cls, data: dict) -> "Hook":
        return cls(
            name=data["name"],
            command=data["command"],
            output=data.get("output", False),
            timeout=data.get("timeout"),
            env=data.get("env"),
        )


@dataclass
class Rule:
    src: Path
    targets: list[str]
    to: str
    on_start: list[Hook] = field(default_factory=list)
    on_end: list[Hook] = field(default_factory=list)

    @classmethod
    def from_dict(cls, data: dict) -> "Rule":
        return cls(
            src=Path(data["from"]).expanduser().absolute(),
            targets=data["targets"],
            to=data["to"],
            on_start=[Hook.from_dict(h) for h in data.get("on-start", [])],
            on_end=[Hook.from_dict(h) for h in data.get("on-end", [])],
        )


@dataclass
class GlobalHooks:
    pre_backup: list[Hook] = field(default_factory=list)
    post_backup: list[Hook] = field(default_factory=list)
    pre_restore: list[Hook] = field(default_factory=list)
    post_restore: list[Hook] = field(default_factory=list)

    @classmethod
    def from_dict(cls, data: dict) -> "GlobalHooks":
        return cls(
            pre_backup=[Hook.from_dict(h) for h in data.get("pre-backup", [])],
            post_backup=[Hook.from_dict(h) for h in data.get("post-backup", [])],
            pre_restore=[Hook.from_dict(h) for h in data.get("pre-restore", [])],
            post_restore=[Hook.from_dict(h) for h in data.get("post-restore", [])],
        )


@dataclass
class Config:
    destination: Path
    rules: list[Rule]
    hooks: GlobalHooks | None = None
    clean: bool = False
    metadata: bool = True

    @classmethod
    def from_dict(cls, data: dict) -> "Config":
        return cls(
            destination=Path(data["destination"]).expanduser().absolute(),
            rules=[Rule.from_dict(r) for r in data["rules"]],
            hooks=GlobalHooks.from_dict(data.get("hooks", {})),
            clean=data.get("clean", False),
            metadata=data.get("metadata", True),
        )


def read_config(file_path: str, quiet: bool) -> Config | None:
    """
    Read the configuration from a YAML file.

    Returns:
        A Config object if the configuration is valid, None otherwise.
    """

    try:
        with open(file_path, "r") as file:
            yaml_data = yaml.safe_load(file)
            os.chdir(
                Path(file_path).parent
            )  # Change working directory to config file's directory
            config = Config.from_dict(yaml_data)

            # Check whether warnings should be suppressed
            if not quiet:
                if not _warn(config):
                    return

            return config
    except Exception as e:
        rprint_error(f"Error reading config file: {e}")

    return None


def _warn(cfg: Config) -> bool:
    """
    Warns and prompts if hooks are present in the config.

    Returns True if safe to continue, False otherwise.
    """

    if _has_hooks(cfg):
        rprint_warning("Hooks detected in configuration.")
        rprint_warning(
            "Please ensure the safety of commands in hooks before execution."
        )
        return click.confirm("Continue?", default=False)
    return True


def _has_hooks(cfg: Config) -> bool:
    """
    Efficiently check if a configuration contains any hooks without building the full hook dictionary.
    """

    # Check rule-level hooks first
    for rule in cfg.rules:
        if rule.on_start and len(rule.on_start) > 0:
            return True
        if rule.on_end and len(rule.on_end) > 0:
            return True

    # Check global hooks if needed
    if cfg.hooks:
        if (
            cfg.hooks.pre_backup
            and len(cfg.hooks.pre_backup) > 0
            or cfg.hooks.post_backup
            and len(cfg.hooks.post_backup) > 0
            or cfg.hooks.pre_restore
            and len(cfg.hooks.pre_restore) > 0
            or cfg.hooks.post_restore
            and len(cfg.hooks.post_restore) > 0
        ):
            return True

    return False
