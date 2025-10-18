import json
import subprocess
from unittest.mock import MagicMock, patch

from cockup.src.list import _process_cask, get_zap_dict


class TestProcessCask:
    """Test the _process_cask function."""

    def test_process_cask_with_zap_section(self):
        """Test processing a cask with a zap section."""
        mock_output = json.dumps(
            {
                "casks": [
                    {
                        "token": "test-app",
                        "name": ["TestApp"],
                        "artifacts": [
                            {
                                "zap": [
                                    {
                                        "trash": [
                                            "~/Library/Application Support/TestApp",
                                            "~/Library/Caches/TestApp",
                                        ]
                                    }
                                ]
                            }
                        ],
                    }
                ]
            }
        )

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout=mock_output)
            cask, zap_items = _process_cask("test-app")

        assert cask == "test-app"
        assert "~/Library/Application Support/TestApp" in zap_items
        assert "~/Library/Caches/TestApp" in zap_items

    def test_process_cask_no_zap_section(self):
        """Test processing a cask without zap section."""
        mock_output = json.dumps(
            {"casks": [{"token": "no-zap-app", "name": ["NoZapApp"]}]}
        )

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout=mock_output)
            cask, zap_items = _process_cask("no-zap-app")

        assert cask == "no-zap-app"
        assert zap_items == []

    def test_process_cask_command_error(self):
        """Test handling of command execution errors."""
        with patch(
            "subprocess.run",
            side_effect=subprocess.CalledProcessError(1, ["brew", "cat"]),
        ):
            cask, zap_items = _process_cask("error-app")

        assert cask == "error-app"
        assert zap_items == []

    def test_process_cask_generic_exception(self):
        """Test handling of generic exceptions."""
        with patch("subprocess.run", side_effect=Exception("Generic error")):
            cask, zap_items = _process_cask("exception-app")

        assert cask == "exception-app"
        assert zap_items == []

    def test_process_cask_complex_zap_section(self):
        """Test processing a cask with complex zap section."""
        mock_output = json.dumps(
            {
                "casks": [
                    {
                        "token": "complex-app",
                        "name": ["ComplexApp"],
                        "artifacts": [
                            {
                                "zap": [
                                    {
                                        "trash": [
                                            "~/Library/Application Support/ComplexApp",
                                            "~/Library/Caches/com.example.complexapp",
                                            "~/Library/HTTPStorages/com.example.complexapp",
                                        ]
                                    }
                                ]
                            }
                        ],
                    }
                ]
            }
        )

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout=mock_output)
            cask, zap_items = _process_cask("complex-app")

        assert cask == "complex-app"
        assert len(zap_items) >= 3
        assert "~/Library/Application Support/ComplexApp" in zap_items

    def test_process_cask_empty_zap_section(self):
        """Test processing a cask with empty zap section."""
        mock_output = json.dumps(
            {"casks": [{"token": "empty-zap", "name": ["EmptyZap"], "zap": []}]}
        )

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout=mock_output)
            cask, zap_items = _process_cask("empty-zap")

        assert cask == "empty-zap"
        assert zap_items == []

    def test_process_cask_subprocess_parameters(self):
        """Test that subprocess.run is called with correct parameters."""
        mock_output = '{"casks": [{"token": "test", "name": ["Test"]}]}'

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout=mock_output)
            _process_cask("test-cask")

        mock_run.assert_called_once()
        call_args = mock_run.call_args
        assert call_args[0][0] == ["brew", "info", "--json=v2", "--cask", "test-cask"]
        assert call_args[1]["capture_output"] is True
        assert call_args[1]["text"] is True


class TestGetZapDict:
    """Test the get_zap_dict function."""

    def test_get_zap_dict_success(self):
        """Test successful get_zap_dict execution."""
        mock_cask_list = "firefox\nchrome\nvscode"

        # Mock the results from _process_cask
        def mock_process_cask_side_effect(cask):
            if cask == "firefox":
                return cask, ["~/Library/Application Support/Firefox"]
            elif cask == "chrome":
                return cask, ["~/Library/Application Support/Google/Chrome"]
            elif cask == "vscode":
                return cask, []  # No zap items
            return cask, []

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout=mock_cask_list)

            with patch(
                "cockup.src.list._process_cask",
                side_effect=mock_process_cask_side_effect,
            ):
                result = get_zap_dict()

        # Only casks with zap items should be in result
        assert "firefox" in result
        assert "chrome" in result
        assert "vscode" not in result
        assert result["firefox"] == ["~/Library/Application Support/Firefox"]

    def test_get_zap_dict_no_casks(self):
        """Test get_zap_dict when no casks are installed."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout="")
            result = get_zap_dict()

        assert result == {}

    def test_get_zap_dict_command_error(self):
        """Test get_zap_dict when brew list command fails."""
        with patch(
            "subprocess.run",
            side_effect=subprocess.CalledProcessError(1, ["brew", "list"]),
        ):
            result = get_zap_dict()

        assert result == {}

    def test_get_zap_dict_generic_exception(self):
        """Test get_zap_dict with generic exception."""
        with patch("subprocess.run", side_effect=Exception("Generic error")):
            result = get_zap_dict()

        assert result == {}

    def test_get_zap_dict_empty_stdout(self):
        """Test get_zap_dict with empty stdout."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout="   ")  # Whitespace only
            result = get_zap_dict()

        assert result == {}

    def test_get_zap_dict_filters_empty_zap_items(self):
        """Test that get_zap_dict filters out casks with no zap items."""
        mock_cask_list = "has-zap\nno-zap"

        def mock_process_cask_side_effect(cask):
            if cask == "has-zap":
                return cask, ["~/Library/Application Support/HasZap"]
            else:
                return cask, []  # No zap items

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout=mock_cask_list)

            with patch(
                "cockup.src.list._process_cask",
                side_effect=mock_process_cask_side_effect,
            ):
                result = get_zap_dict()

        assert "has-zap" in result
        assert "no-zap" not in result
        assert len(result) == 1

    def test_get_zap_dict_subprocess_parameters(self):
        """Test that subprocess.run is called with correct parameters."""
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout="")
            get_zap_dict()

        # Should be called twice: once for brew --version, once for brew list --casks
        assert mock_run.call_count == 2

        # First call should be brew --version (from _is_brew_installed)
        first_call = mock_run.call_args_list[0]
        assert first_call[0][0] == ["brew", "--version"]

        # Second call should be brew list --casks
        second_call = mock_run.call_args_list[1]
        assert second_call[0][0] == ["brew", "list", "--casks"]
