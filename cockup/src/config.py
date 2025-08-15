import os
from pathlib import Path

import click
import yaml
from pydantic import BaseModel, ConfigDict, Field, ValidationError, field_validator

from cockup.src.console import rprint, rprint_error, rprint_warning


class ConfigModel(BaseModel):
    model_config = ConfigDict(validate_by_name=True)


class Hook(ConfigModel):
    name: str
    command: list[str]
    output: bool = False
    timeout: int = 10


class Rule(ConfigModel):
    src: Path = Field(validation_alias="from")
    targets: list[str]
    to: str
    on_start: list[Hook] = Field(default=[], validation_alias="on-start")
    on_end: list[Hook] = Field(default=[], validation_alias="on-end")

    @field_validator("src")
    @classmethod
    def expand_src_path(cls, v: Path):
        return v.expanduser().absolute()


class GlobalHooks(ConfigModel):
    pre_backup: list[Hook] = Field(default=[], validation_alias="pre-backup")
    post_backup: list[Hook] = Field(default=[], validation_alias="post-backup")
    pre_restore: list[Hook] = Field(default=[], validation_alias="pre-restore")
    post_restore: list[Hook] = Field(default=[], validation_alias="post-restore")


class Config(ConfigModel):
    destination: Path
    rules: list[Rule]
    hooks: GlobalHooks | None = None
    clean: bool = False
    metadata: bool = True

    @field_validator("destination")
    @classmethod
    def expand_destination_path(cls, v: Path):
        return v.expanduser().absolute()


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
            config = Config.model_validate(yaml_data)

            # Check whether warnings should be suppressed
            if not quiet:
                if not _warn(config):
                    return

            return config
    except ValidationError as e:
        rprint_error("Error in config file:\n")
        for error in e.errors():
            rprint_error(f"Location: {' -> '.join(str(loc) for loc in error['loc'])}")
            rprint_error(f"Message: {error['msg']}")
            rprint()
    except Exception as e:
        rprint_error(f"Error reading YAML file: {e}")

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
