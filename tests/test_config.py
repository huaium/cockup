import tempfile
from pathlib import Path

import yaml

from cockup.src.config import Config, Hooks, Rule, _read_yaml, read_config


class TestReadYaml:
    """Test the _read_yaml helper function."""

    def test_read_valid_yaml(self):
        """Test reading a valid YAML file."""
        yaml_content = {
            "destination": "~/backup",
            "rules": [{"from": "~/Documents", "targets": ["*.txt"], "to": "docs"}],
            "clean": True,
            "metadata": False,
            "hooks": {"pre-backup": [{"name": "test", "command": ["echo", "test"]}]},
        }

        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            yaml.dump(yaml_content, f)
            config_file = f.name

        result = _read_yaml(config_file)
        Path(config_file).unlink()  # Clean up
        assert result == yaml_content

    def test_read_nonexistent_file(self, capsys):
        """Test reading a non-existent YAML file."""
        result = _read_yaml("nonexistent.yaml")

        assert result == {}
        captured = capsys.readouterr()
        assert "Error reading YAML file" in captured.out

    def test_read_invalid_yaml(self, capsys):
        """Test reading invalid YAML content."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            f.write("invalid: yaml: content:")
            config_file = f.name

        result = _read_yaml(config_file)
        Path(config_file).unlink()  # Clean up

        assert result == {}
        captured = capsys.readouterr()
        assert "Error reading YAML file" in captured.out


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

        config = read_config(config_file)
        Path(config_file).unlink()  # Clean up

        assert config is not None
        assert isinstance(config, Config)
        assert config.clean is True
        assert config.metadata is False
        assert len(config.rules) == 2
        assert isinstance(config.rules[0], Rule)
        assert config.rules[0].targets == ["file1.txt", "file2.txt"]
        assert config.rules[0].to == "docs"
        assert len(config.hooks.pre_backup) == 1
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

        config = read_config(config_file)
        Path(config_file).unlink()  # Clean up

        assert config is not None
        assert isinstance(config, Config)
        assert config.clean is False  # Default value
        assert config.metadata is True  # Default value
        assert len(config.rules) == 1
        assert isinstance(config.rules[0], Rule)
        assert len(config.hooks.pre_backup) == 0

    def test_read_config_missing_destination(self, capsys):
        """Test handling of config without destination."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            config_content = {
                "rules": [{"from": "~/Documents", "targets": ["*.txt"], "to": "docs"}]
            }
            yaml.dump(config_content, f)
            config_file = f.name

        config = read_config(config_file)
        Path(config_file).unlink()  # Clean up

        assert config is None
        captured = capsys.readouterr()
        assert "No destination specified" in captured.out

    def test_read_config_missing_rules(self, capsys):
        """Test handling of config without rules."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            config_content = {"destination": "~/backup"}
            yaml.dump(config_content, f)
            config_file = f.name

        config = read_config(config_file)
        Path(config_file).unlink()  # Clean up

        assert config is None
        captured = capsys.readouterr()
        assert "No rules specified" in captured.out

    def test_read_config_rule_vs_rules_hint(self, capsys):
        """Test hint when user uses 'rule' instead of 'rules'."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            config_content = {
                "destination": "~/backup",
                "rule": "~/Documents",  # Wrong key
            }
            yaml.dump(config_content, f)
            config_file = f.name

        config = read_config(config_file)
        Path(config_file).unlink()  # Clean up

        assert config is None
        captured = capsys.readouterr()
        assert "No rules specified" in captured.out
        assert "Did you mistakenly use `rule` instead of `rules`?" in captured.out

    def test_read_config_invalid_rule_format(self, capsys):
        """Test handling of invalid rule format."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            config_content = {
                "destination": "~/backup",
                "rules": ["not a dict"],  # Should be dict with from/targets/to
            }
            yaml.dump(config_content, f)
            config_file = f.name

        config = read_config(config_file)
        Path(config_file).unlink()  # Clean up

        assert config is None
        captured = capsys.readouterr()
        assert "Each rule must be a dictionary" in captured.out

    def test_read_config_missing_rule_fields(self, capsys):
        """Test handling of rules missing required fields."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            config_content = {
                "destination": "~/backup",
                "rules": [{"from": "~/Documents"}],  # Missing targets and to
            }
            yaml.dump(config_content, f)
            config_file = f.name

        config = read_config(config_file)
        Path(config_file).unlink()  # Clean up

        assert config is None
        captured = capsys.readouterr()
        assert "missing `targets`" in captured.out

    def test_read_nonexistent_config(self):
        """Test handling of non-existent config file."""
        config = read_config("nonexistent.yaml")
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

        config = read_config(config_file)
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

        config = read_config(config_file)
        Path(config_file).unlink()  # Clean up

        assert config is not None
        assert config.clean is False
        assert config.metadata is True
        assert len(config.hooks.pre_backup) == 0

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

        config = read_config(config_file)
        Path(config_file).unlink()  # Clean up

        assert config is not None
        assert config.clean is True
        assert config.metadata is False
        assert len(config.hooks.pre_backup) == 1

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

        config = read_config(config_file)
        Path(config_file).unlink()  # Clean up

        assert config is not None
        assert len(config.rules[0].on_start) == 1
        assert len(config.rules[0].on_end) == 1


class TestConfig:
    """Test the Config dataclass."""

    def test_config_initialization(self):
        """Test Config object initialization."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            dest = Path(tmp_dir) / "backup"
            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            rules = [rule]
            hooks = Hooks(
                pre_backup=[{"name": "test", "command": ["echo", "test"]}],
                post_backup=[],
                pre_restore=[],
                post_restore=[],
            )

            config = Config(
                destination=dest, rules=rules, hooks=hooks, clean=True, metadata=False
            )

            assert config.destination == dest
            assert config.rules == rules
            assert config.hooks == hooks
            assert config.clean is True
            assert config.metadata is False


class TestRuleDataclass:
    """Test the Rule dataclass."""

    def test_rule_initialization(self):
        """Test Rule object initialization."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            src = Path(tmp_dir) / "source"
            targets = ["*.txt", "*.pdf"]
            to = "documents"
            on_start = [{"name": "start", "command": ["echo", "starting"]}]
            on_end = [{"name": "end", "command": ["echo", "done"]}]

            rule = Rule(
                src=src, targets=targets, to=to, on_start=on_start, on_end=on_end
            )

            assert rule.src == src
            assert rule.targets == targets
            assert rule.to == to
            assert rule.on_start == on_start
            assert rule.on_end == on_end


class TestHooksDataclass:
    """Test the Hooks dataclass."""

    def test_hooks_initialization(self):
        """Test Hooks object initialization."""
        pre_backup = [{"name": "pre_backup", "command": ["echo", "pre"]}]
        post_backup = [{"name": "post_backup", "command": ["echo", "post"]}]
        pre_restore = [{"name": "pre_restore", "command": ["echo", "pre_restore"]}]
        post_restore = [{"name": "post_restore", "command": ["echo", "post_restore"]}]

        hooks = Hooks(
            pre_backup=pre_backup,
            post_backup=post_backup,
            pre_restore=pre_restore,
            post_restore=post_restore,
        )

        assert hooks.pre_backup == pre_backup
        assert hooks.post_backup == post_backup
        assert hooks.pre_restore == pre_restore
        assert hooks.post_restore == post_restore
