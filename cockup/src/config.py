from pathlib import Path

import yaml
from pydantic import BaseModel, ConfigDict, Field, ValidationError, field_validator

from cockup.src.console import rprint, rprint_error


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
    def expand_src_path(cls, v):
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
    def expand_destination_path(cls, v):
        return v.expanduser().absolute()


def read_config(file_path: str) -> Config | None:
    """
    Read the configuration from a YAML file.

    Returns:
        A Config object if the configuration is valid, None otherwise.
    """

    try:
        with open(file_path, "r") as file:
            yaml_data = yaml.safe_load(file)
            config = Config.model_validate(yaml_data)
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
