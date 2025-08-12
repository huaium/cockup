import subprocess
from unittest.mock import MagicMock, patch

from cockup.src.hooks import run_hooks


class TestRunHooks:
    """Test the run_hooks function."""

    def test_run_hooks_success(self, capsys):
        """Test successful execution of hooks."""
        hooks = [
            {"name": "test_hook_1", "command": ["echo", "test1"]},
            {"name": "test_hook_2", "command": ["echo", "test2"], "output": True},
        ]

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks(hooks)

        # Should have been called twice
        assert mock_run.call_count == 2

        # Check first call (no output)
        first_call = mock_run.call_args_list[0]
        assert first_call[0][0] == ["echo", "test1"]
        assert first_call[1]["capture_output"] is True  # not output = capture_output
        assert first_call[1]["check"] is True

        # Check second call (with output)
        second_call = mock_run.call_args_list[1]
        assert second_call[0][0] == ["echo", "test2"]
        assert second_call[1]["capture_output"] is False  # output = not capture_output

        # Check console output
        captured = capsys.readouterr()
        assert "Running hook (1/2): test_hook_1" in captured.out
        assert "Running hook (2/2): test_hook_2" in captured.out
        assert "Completed 2/2 hooks successfully" in captured.out

    def test_run_hooks_empty_list(self, capsys):
        """Test run_hooks with empty list."""
        run_hooks([])

        captured = capsys.readouterr()
        assert "Completed 0/0 hook successfully" in captured.out

    def test_run_hooks_missing_name(self, capsys):
        """Test hook without name field."""
        hooks = [{"command": ["echo", "test"]}]  # Missing name

        with patch("subprocess.run"):
            run_hooks(hooks)

        captured = capsys.readouterr()
        assert "Hook 1 missing `name`, skipping..." in captured.out
        assert "Completed 0/1 hook successfully" in captured.out

    def test_run_hooks_missing_command(self, capsys):
        """Test hook without command field."""
        hooks = [{"name": "test_hook"}]  # Missing command

        with patch("subprocess.run"):
            run_hooks(hooks)

        captured = capsys.readouterr()
        assert "Hook 1 missing `command`, skipping..." in captured.out
        assert "Completed 0/1 hook successfully" in captured.out

    def test_run_hooks_timeout(self, capsys):
        """Test hook timeout handling."""
        hooks = [{"name": "timeout_hook", "command": ["sleep", "100"], "timeout": 1}]

        with patch(
            "subprocess.run", side_effect=subprocess.TimeoutExpired(["sleep", "100"], 1)
        ):
            run_hooks(hooks)

        captured = capsys.readouterr()
        assert "Command `timeout_hook` timed out after 1 seconds" in captured.out
        assert "Completed 0/1 hook successfully" in captured.out

    def test_run_hooks_command_failure(self, capsys):
        """Test handling of command execution failure."""
        hooks = [{"name": "failing_hook", "command": ["false"]}]

        with patch(
            "subprocess.run", side_effect=subprocess.CalledProcessError(1, ["false"])
        ):
            run_hooks(hooks)

        captured = capsys.readouterr()
        assert "Error executing command `failing_hook`" in captured.out
        assert "Completed 0/1 hook successfully" in captured.out

    def test_run_hooks_generic_exception(self, capsys):
        """Test handling of generic exceptions."""
        hooks = [{"name": "exception_hook", "command": ["echo", "test"]}]

        with patch("subprocess.run", side_effect=Exception("Generic error")):
            run_hooks(hooks)

        captured = capsys.readouterr()
        assert "Error executing command `exception_hook`: Generic error" in captured.out
        assert "Completed 0/1 hook successfully" in captured.out

    def test_run_hooks_default_values(self):
        """Test default values for optional hook parameters."""
        hooks = [{"name": "default_hook", "command": ["echo", "test"]}]

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks(hooks)

        # Check that defaults were applied
        call_args = mock_run.call_args_list[0][1]
        assert call_args["timeout"] == 10  # Default timeout
        assert (
            call_args["capture_output"] is True
        )  # Default output=False -> capture_output=True

    def test_run_hooks_custom_timeout(self):
        """Test custom timeout value."""
        hooks = [{"name": "custom_timeout", "command": ["echo", "test"], "timeout": 30}]

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks(hooks)

        call_args = mock_run.call_args_list[0][1]
        assert call_args["timeout"] == 30

    def test_run_hooks_mixed_success_failure(self, capsys):
        """Test mixed success and failure scenarios."""
        hooks = [
            {"name": "success_hook", "command": ["echo", "success"]},
            {"name": "fail_hook", "command": ["false"]},
            {"name": "success_hook_2", "command": ["echo", "success2"]},
        ]

        def mock_run_side_effect(command, **kwargs):
            if command == ["false"]:
                raise subprocess.CalledProcessError(1, command)
            return MagicMock()

        with patch("subprocess.run", side_effect=mock_run_side_effect):
            run_hooks(hooks)

        captured = capsys.readouterr()
        assert "Completed 2/3 hooks successfully" in captured.out
        assert "Error executing command `fail_hook`" in captured.out

    def test_run_hooks_output_flag_combinations(self):
        """Test different output flag values."""
        test_cases = [
            (True, False),  # output=True -> capture_output=False
            (False, True),  # output=False -> capture_output=True
            (None, True),  # output not specified -> capture_output=True (default)
        ]

        for output_value, expected_capture in test_cases:
            hook = {"name": "test", "command": ["echo", "test"]}
            if output_value is not None:
                hook["output"] = output_value

            with patch("subprocess.run") as mock_run:
                mock_run.return_value = MagicMock()
                run_hooks([hook])

            call_args = mock_run.call_args_list[0][1]
            assert call_args["capture_output"] == expected_capture

    def test_run_hooks_text_parameter(self):
        """Test that text=True is always passed to subprocess.run."""
        hooks = [{"name": "text_test", "command": ["echo", "test"]}]

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks(hooks)

        call_args = mock_run.call_args_list[0][1]
        assert call_args["text"] is True

    def test_run_hooks_check_parameter(self):
        """Test that check=True is always passed to subprocess.run."""
        hooks = [{"name": "check_test", "command": ["echo", "test"]}]

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks(hooks)

        call_args = mock_run.call_args_list[0][1]
        assert call_args["check"] is True

    def test_run_hooks_singular_plural_output(self, capsys):
        """Test correct singular/plural in completion message."""
        # Test singular
        hooks = [{"name": "single_hook", "command": ["echo", "test"]}]

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks(hooks)

        captured = capsys.readouterr()
        assert "Completed 1/1 hook successfully" in captured.out

        # Test plural
        hooks = [
            {"name": "hook_1", "command": ["echo", "test1"]},
            {"name": "hook_2", "command": ["echo", "test2"]},
        ]

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks(hooks)

        captured = capsys.readouterr()
        assert "Completed 2/2 hooks successfully" in captured.out

    def test_run_hooks_complex_commands(self):
        """Test hooks with complex command arrays."""
        hooks = [
            {
                "name": "complex_cmd",
                "command": ["python", "-c", "print('hello world')"],
            },
            {"name": "multiarg_cmd", "command": ["ls", "-la", "/tmp"]},
        ]

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks(hooks)

        # Verify commands are passed through correctly
        first_call = mock_run.call_args_list[0][0][0]
        assert first_call == ["python", "-c", "print('hello world')"]

        second_call = mock_run.call_args_list[1][0][0]
        assert second_call == ["ls", "-la", "/tmp"]
