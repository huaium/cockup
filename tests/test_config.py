import tempfile
from pathlib import Path
from unittest.mock import patch

import yaml

from cockup.src.config import Config, Rule, read_config


class TestReadConfig:
    """Test the read_config function."""

    def test_read_valid_config(self):
        """Test reading a valid configuration file."""
        config_content = {
            "destination": "~/backup",
            "rules": [
                {
                    "from": "~/Documents",
                    "targets": ["file1.txt", "file2.txt"],
                    "to": "docs",
                },
                {"from": "~/Downloads", "targets": ["*.zip"], "to": "downloads"},
            ],
            "clean": True,
            "metadata": False,
            "hooks": {"pre-backup": [{"name": "test", "command": ["echo", "test"]}]},
        }

        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            yaml.dump(config_content, f)
            config_file = f.name

        with patch("click.confirm", return_value=True):
            config = read_config(config_file, quiet=False)
        Path(config_file).unlink()  # Clean up

        assert config is not None
        assert isinstance(config, Config)
        assert config.clean is True
        assert config.metadata is False
        assert len(config.rules) == 2
        assert isinstance(config.rules[0], Rule)
        assert config.rules[0].targets == ["file1.txt", "file2.txt"]
        assert config.rules[0].to == "docs"
        assert config.hooks is not None and len(config.hooks.pre_backup) == 1
        assert config.destination.name == "backup"

    def test_read_minimal_config(self):
        """Test reading a minimal valid configuration."""
        config_content = {
            "destination": "~/backup",
            "rules": [{"from": "~/Documents", "targets": ["*.txt"], "to": "docs"}],
        }

        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            yaml.dump(config_content, f)
            config_file = f.name

        config = read_config(config_file, quiet=True)
        Path(config_file).unlink()  # Clean up

        assert config is not None
        assert isinstance(config, Config)
        assert config.clean is False  # Default value
        assert config.metadata is True  # Default value
        assert len(config.rules) == 1
        assert isinstance(config.rules[0], Rule)
        assert config.hooks is None or len(config.hooks.pre_backup) == 0

    def test_read_config_missing_destination(self, capsys):
        """Test handling of config without destination."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            config_content = {
                "rules": [{"from": "~/Documents", "targets": ["*.txt"], "to": "docs"}]
            }
            yaml.dump(config_content, f)
            config_file = f.name

        config = read_config(config_file, quiet=True)
        Path(config_file).unlink()  # Clean up

        assert config is None
        captured = capsys.readouterr()
        assert "Error reading config file" in captured.out

    def test_read_config_missing_rules(self, capsys):
        """Test handling of config without rules."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            config_content = {"destination": "~/backup"}
            yaml.dump(config_content, f)
            config_file = f.name

        config = read_config(config_file, quiet=True)
        Path(config_file).unlink()  # Clean up

        assert config is None
        captured = capsys.readouterr()
        assert "Error reading config file" in captured.out

    def test_read_config_rule_vs_rules_hint(self, capsys):
        """Test hint when user uses 'rule' instead of 'rules'."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            config_content = {
                "destination": "~/backup",
                "rule": "~/Documents",  # Wrong key
            }
            yaml.dump(config_content, f)
            config_file = f.name

        config = read_config(config_file, quiet=True)
        Path(config_file).unlink()  # Clean up

        assert config is None
        captured = capsys.readouterr()
        assert "Error reading config file" in captured.out

    def test_read_config_invalid_rule_format(self, capsys):
        """Test handling of invalid rule format."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            config_content = {
                "destination": "~/backup",
                "rules": ["not a dict"],  # Should be dict with from/targets/to
            }
            yaml.dump(config_content, f)
            config_file = f.name

        config = read_config(config_file, quiet=True)
        Path(config_file).unlink()  # Clean up

        assert config is None
        captured = capsys.readouterr()
        assert "Error reading config file" in captured.out

    def test_read_config_missing_rule_fields(self, capsys):
        """Test handling of rules missing required fields."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            config_content = {
                "destination": "~/backup",
                "rules": [{"from": "~/Documents"}],  # Missing targets and to
            }
            yaml.dump(config_content, f)
            config_file = f.name

        config = read_config(config_file, quiet=True)
        Path(config_file).unlink()  # Clean up

        assert config is None
        captured = capsys.readouterr()
        assert "Error reading config file" in captured.out

    def test_read_nonexistent_config(self):
        """Test handling of non-existent config file."""
        config = read_config("nonexistent.yaml", quiet=True)
        assert config is None

    def test_config_path_expansion(self):
        """Test that paths are properly expanded."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            config_content = {
                "destination": "~/backup",
                "rules": [{"from": "~/Documents", "targets": ["*.txt"], "to": "docs"}],
            }
            yaml.dump(config_content, f)
            config_file = f.name

        config = read_config(config_file, quiet=True)
        Path(config_file).unlink()  # Clean up

        assert config is not None
        assert config.destination.is_absolute()
        assert config.rules[0].src.is_absolute()

    def test_config_defaults(self):
        """Test that configuration defaults are applied correctly."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            config_content = {
                "destination": "~/backup",
                "rules": [{"from": "~/Documents", "targets": ["*.txt"], "to": "docs"}],
                # No clean, metadata, or hooks specified
            }
            yaml.dump(config_content, f)
            config_file = f.name

        config = read_config(config_file, quiet=True)
        Path(config_file).unlink()  # Clean up

        assert config is not None
        assert config.clean is False
        assert config.metadata is True
        assert config.hooks is None or len(config.hooks.pre_backup) == 0

    def test_config_explicit_values(self):
        """Test explicit configuration values override defaults."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            config_content = {
                "destination": "~/backup",
                "rules": [{"from": "~/Documents", "targets": ["*.txt"], "to": "docs"}],
                "clean": True,
                "metadata": False,
                "hooks": {
                    "pre-backup": [{"name": "test", "command": ["echo", "test"]}]
                },
            }
            yaml.dump(config_content, f)
            config_file = f.name

        with patch("click.confirm", return_value=True):
            config = read_config(config_file, quiet=False)
        Path(config_file).unlink()  # Clean up

        assert config is not None
        assert config.clean is True
        assert config.metadata is False
        assert config.hooks is not None and len(config.hooks.pre_backup) == 1

    def test_config_with_rule_hooks(self):
        """Test config with rule-specific hooks."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            config_content = {
                "destination": "~/backup",
                "rules": [
                    {
                        "from": "~/Documents",
                        "targets": ["*.txt"],
                        "to": "docs",
                        "on-start": [{"name": "pre", "command": ["echo", "before"]}],
                        "on-end": [{"name": "post", "command": ["echo", "after"]}],
                    }
                ],
            }
            yaml.dump(config_content, f)
            config_file = f.name

        with patch("click.confirm", return_value=True):
            config = read_config(config_file, quiet=False)
        Path(config_file).unlink()  # Clean up

        assert config is not None
        assert len(config.rules[0].on_start) == 1
        assert len(config.rules[0].on_end) == 1
