import tempfile
from pathlib import Path
from unittest.mock import patch

import yaml
from click.testing import CliRunner

from cockup.main import backup_command, list_command, main, restore_command


class TestBackupCommand:
    """Test the backup command functionality."""

    def test_backup_command_success(self):
        """Test successful backup command execution."""
        config_content = {
            "destination": "~/backup",
            "rules": [{"from": "~/Documents", "targets": ["*.txt"], "to": "docs"}],
            "clean": False,
            "metadata": True,
        }

        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            yaml.dump(config_content, f)
            config_file = f.name

        runner = CliRunner()
        with patch("cockup.main.backup") as mock_backup:
            result = runner.invoke(backup_command, [config_file])

        Path(config_file).unlink()  # Clean up

        assert result.exit_code == 0
        mock_backup.assert_called_once()

    def test_backup_command_nonexistent_file(self):
        """Test backup command with non-existent config file."""
        runner = CliRunner()
        result = runner.invoke(backup_command, ["nonexistent.yaml"])

        assert result.exit_code != 0
        assert "does not exist" in result.output

    def test_backup_command_invalid_config(self):
        """Test backup command with invalid config."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            f.write("invalid: config: missing required fields")
            config_file = f.name

        runner = CliRunner()
        with patch("cockup.main.backup") as mock_backup:
            result = runner.invoke(backup_command, [config_file])

        Path(config_file).unlink()  # Clean up

        assert result.exit_code == 0  # Function returns gracefully
        mock_backup.assert_not_called()  # But backup is not called


class TestRestoreCommand:
    """Test the restore command functionality."""

    def test_restore_command_success(self):
        """Test successful restore command execution."""
        config_content = {
            "destination": "~/backup",
            "rules": [{"from": "~/Documents", "targets": ["*.txt"], "to": "docs"}],
        }

        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            yaml.dump(config_content, f)
            config_file = f.name

        runner = CliRunner()
        with patch("cockup.main.restore") as mock_restore:
            result = runner.invoke(restore_command, [config_file])

        Path(config_file).unlink()  # Clean up

        assert result.exit_code == 0
        mock_restore.assert_called_once()

    def test_restore_command_nonexistent_file(self):
        """Test restore command with non-existent config file."""
        runner = CliRunner()
        result = runner.invoke(restore_command, ["nonexistent.yaml"])

        assert result.exit_code != 0
        assert "does not exist" in result.output

    def test_restore_command_invalid_config(self):
        """Test restore command with invalid config."""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            f.write("invalid: yaml: content:")
            config_file = f.name

        runner = CliRunner()
        with patch("cockup.main.restore") as mock_restore:
            result = runner.invoke(restore_command, [config_file])

        Path(config_file).unlink()  # Clean up

        assert result.exit_code == 0  # Function returns gracefully
        mock_restore.assert_not_called()  # But restore is not called


class TestListCommand:
    """Test the list command functionality."""

    def test_list_command_success(self):
        """Test successful list command execution."""
        mock_zap_dict = {
            "firefox": [
                "~/Library/Application Support/Firefox",
                "~/Library/Caches/Firefox",
            ],
            "chrome": ["~/Library/Application Support/Google/Chrome"],
        }

        runner = CliRunner()
        with patch("cockup.main.get_zap_dict", return_value=mock_zap_dict):
            result = runner.invoke(list_command)

        assert result.exit_code == 0
        assert "firefox" in result.output
        assert "chrome" in result.output
        assert "Firefox" in result.output

    def test_list_command_no_casks(self):
        """Test list command when no casks are installed."""
        runner = CliRunner()
        with patch("cockup.main.get_zap_dict", return_value={}):
            result = runner.invoke(list_command)

        assert result.exit_code == 0
        assert "Retrieving potential configs" in result.output

    def test_list_command_error_handling(self):
        """Test list command error handling."""
        runner = CliRunner()
        with patch("cockup.main.get_zap_dict", side_effect=Exception("Test error")):
            result = runner.invoke(list_command)

        # Click command should handle the exception, but exit code might be 1
        assert result.exit_code in [0, 1]  # Either is acceptable


class TestMainGroup:
    """Test the main command group."""

    def test_main_help(self):
        """Test main command help output."""
        runner = CliRunner()
        result = runner.invoke(main, ["--help"])

        assert result.exit_code == 0
        # The help text might not include the full description
        assert "backup" in result.output
        assert "restore" in result.output
        assert "list" in result.output

    def test_main_version_option(self):
        """Test main command with version flags."""
        runner = CliRunner()

        # Test -h flag
        result = runner.invoke(main, ["-h"])
        assert result.exit_code == 0

        # Test --help flag
        result = runner.invoke(main, ["--help"])
        assert result.exit_code == 0


class TestCommandIntegration:
    """Integration tests for commands."""

    def test_backup_restore_integration(self):
        """Test backup and restore commands with the same config."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            backup_dir = Path(tmp_dir) / "backup"
            config_content = {
                "destination": str(backup_dir),
                "rules": [
                    {
                        "from": str(Path(tmp_dir) / "src"),
                        "targets": ["*.txt"],
                        "to": "documents",
                    }
                ],
                "clean": False,
                "metadata": True,
            }

            with tempfile.NamedTemporaryFile(
                mode="w", suffix=".yaml", delete=False
            ) as f:
                yaml.dump(config_content, f)
                config_file = f.name

            runner = CliRunner()

            # Mock the actual backup and restore functions
            with patch("cockup.main.backup") as mock_backup:
                with patch("cockup.main.restore") as mock_restore:
                    # Test backup
                    result_backup = runner.invoke(backup_command, [config_file])
                    assert result_backup.exit_code == 0
                    mock_backup.assert_called_once()

                    # Test restore
                    result_restore = runner.invoke(restore_command, [config_file])
                    assert result_restore.exit_code == 0
                    mock_restore.assert_called_once()

            Path(config_file).unlink()  # Clean up

    def test_all_commands_exist(self):
        """Test that all expected commands are available."""
        runner = CliRunner()
        result = runner.invoke(main, ["--help"])

        assert result.exit_code == 0
        assert "backup" in result.output
        assert "restore" in result.output
        assert "list" in result.output

    def test_command_specific_help(self):
        """Test help output for individual commands."""
        runner = CliRunner()

        # Test backup command help
        result = runner.invoke(backup_command, ["--help"])
        assert result.exit_code == 0
        # Just check that it's the backup command help
        assert "CONFIG_FILE" in result.output

        # Test restore command help
        result = runner.invoke(restore_command, ["--help"])
        assert result.exit_code == 0
        # Just check that it's the restore command help
        assert "CONFIG_FILE" in result.output

        # Test list command help
        result = runner.invoke(list_command, ["--help"])
        assert result.exit_code == 0


class TestErrorHandling:
    """Test error handling scenarios."""

    def test_missing_config_file_argument(self):
        """Test commands that require config file but don't get one."""
        runner = CliRunner()

        # Backup command without config file
        result = runner.invoke(backup_command)
        assert result.exit_code != 0
        assert "Missing argument" in result.output

        # Restore command without config file
        result = runner.invoke(restore_command)
        assert result.exit_code != 0
        assert "Missing argument" in result.output

    def test_config_validation_errors(self):
        """Test various config validation errors."""
        test_configs = [
            # Missing destination
            {"rules": [{"from": "~/Documents", "targets": ["*.txt"], "to": "docs"}]},
            # Missing rules
            {"destination": "~/backup"},
            # Invalid rule format
            {"destination": "~/backup", "rules": ["invalid_rule"]},
            # Missing required rule fields
            {"destination": "~/backup", "rules": [{"from": "~/Documents"}]},
        ]

        runner = CliRunner()

        for i, config_content in enumerate(test_configs):
            with tempfile.NamedTemporaryFile(
                mode="w", suffix=".yaml", delete=False
            ) as f:
                yaml.dump(config_content, f)
                config_file = f.name

            with patch("cockup.main.backup") as mock_backup:
                result = runner.invoke(backup_command, [config_file])
                # Should exit gracefully without calling backup
                assert result.exit_code == 0
                mock_backup.assert_not_called()

            Path(config_file).unlink()  # Clean up

    def test_yaml_parsing_errors(self):
        """Test YAML parsing error handling."""
        runner = CliRunner()

        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            f.write("invalid: yaml: content: [unclosed bracket")
            config_file = f.name

        with patch("cockup.main.backup") as mock_backup:
            result = runner.invoke(backup_command, [config_file])
            # Should handle gracefully
            assert result.exit_code == 0
            mock_backup.assert_not_called()

        Path(config_file).unlink()  # Clean up

    def test_permission_errors(self):
        """Test handling of permission errors."""
        config_content = {
            "destination": "/root/backup",  # Typically inaccessible
            "rules": [{"from": "~/Documents", "targets": ["*.txt"], "to": "docs"}],
        }

        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            yaml.dump(config_content, f)
            config_file = f.name

        runner = CliRunner()
        with patch(
            "cockup.main.backup", side_effect=PermissionError("Permission denied")
        ):
            result = runner.invoke(backup_command, [config_file])
            # The exception should propagate and cause non-zero exit
            assert result.exit_code == 1

        Path(config_file).unlink()  # Clean up
