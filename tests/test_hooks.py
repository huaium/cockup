import subprocess
from pathlib import Path
from unittest.mock import MagicMock, patch

from cockup.src.config import Config, GlobalHooks, Hook, Rule
from cockup.src.hooks import (
    _get_hook_dict,
    run_hook,
    run_hook_by_name,
    run_hooks,
    run_hooks_with_input,
)


class TestRunHooks:
    """Test the run_hooks function."""

    def test_run_hooks_success(self, capsys):
        """Test successful execution of hooks."""
        hooks = [
            Hook(name="test_hook_1", command=["echo", "test1"]),
            Hook(name="test_hook_2", command=["echo", "test2"], output=True),
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
        assert "Completed 2/2 hooks" in captured.out

    def test_run_hooks_empty_list(self, capsys):
        """Test run_hooks with empty list."""
        run_hooks([])

        captured = capsys.readouterr()
        assert "Completed 0/0 hook" in captured.out

    def test_run_hooks_missing_name(self, capsys):
        """Test hook with empty name field (Pydantic will enforce required field)."""
        # With Pydantic validation, we can't create a Hook without a name
        # But we can test with an empty string name
        hooks = [Hook(name="", command=["echo", "test"])]

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks(hooks)

        captured = capsys.readouterr()
        assert "Running hook (1/1):" in captured.out  # Empty name shows as blank
        assert "Completed 1/1 hook" in captured.out

    def test_run_hooks_command_failure(self, capsys):
        """Test handling of command execution failure and generic exceptions."""
        # Test CalledProcessError
        hooks = [Hook(name="failing_hook", command=["false"])]
        with patch(
            "subprocess.run", side_effect=subprocess.CalledProcessError(1, ["false"])
        ):
            run_hooks(hooks)

        captured = capsys.readouterr()
        assert "Error executing command `failing_hook`" in captured.out
        assert "Completed 0/1 hook" in captured.out

        # Test generic exception
        hooks = [Hook(name="exception_hook", command=["echo", "test"])]
        with patch("subprocess.run", side_effect=Exception("Generic error")):
            run_hooks(hooks)

        captured = capsys.readouterr()
        assert "Error executing command `exception_hook`: Generic error" in captured.out

    def test_run_hooks_default_values(self):
        """Test default values for optional hook parameters."""
        hooks = [Hook(name="default_hook", command=["echo", "test"])]

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks(hooks)

        # Check that defaults were applied
        call_args = mock_run.call_args_list[0][1]
        assert (
            call_args["capture_output"] is True
        )  # Default output=False -> capture_output=True

    def test_run_hooks_custom_timeout(self):
        """Test custom timeout value."""
        hooks = [Hook(name="custom_timeout", command=["echo", "test"], timeout=30)]

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks(hooks)

        call_args = mock_run.call_args_list[0][1]
        assert call_args["timeout"] == 30

    def test_run_hooks_mixed_success_failure(self, capsys):
        """Test mixed success and failure scenarios."""
        hooks = [
            Hook(name="success_hook", command=["echo", "success"]),
            Hook(name="fail_hook", command=["false"]),
            Hook(name="success_hook_2", command=["echo", "success2"]),
        ]

        def mock_run_side_effect(command, **_):
            if command == ["false"]:
                raise subprocess.CalledProcessError(1, command)
            return MagicMock()

        with patch("subprocess.run", side_effect=mock_run_side_effect):
            run_hooks(hooks)

        captured = capsys.readouterr()
        assert "Completed 2/3 hooks" in captured.out
        assert "Error executing command `fail_hook`" in captured.out

    def test_run_hooks_output_flag_combinations(self):
        """Test different output flag values."""
        test_cases = [
            (True, False),  # output=True -> capture_output=False
            (False, True),  # output=False -> capture_output=True
        ]

        for output_value, expected_capture in test_cases:
            hook = Hook(name="test", command=["echo", "test"], output=output_value)

            with patch("subprocess.run") as mock_run:
                mock_run.return_value = MagicMock()
                run_hooks([hook])

            call_args = mock_run.call_args_list[0][1]
            assert call_args["capture_output"] == expected_capture

        # Test default case (output not specified, defaults to False)
        hook = Hook(name="test", command=["echo", "test"])
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks([hook])

        call_args = mock_run.call_args_list[0][1]
        assert (
            call_args["capture_output"] is True
        )  # default output=False -> capture_output=True

    def test_run_hooks_text_parameter(self):
        """Test that text=True is always passed to subprocess.run."""
        hooks = [Hook(name="text_test", command=["echo", "test"])]

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks(hooks)

        call_args = mock_run.call_args_list[0][1]
        assert "text" in call_args
        assert call_args["text"] is True

    def test_run_hooks_check_parameter(self):
        """Test that check=True is always passed to subprocess.run."""
        hooks = [Hook(name="check_test", command=["echo", "test"])]

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks(hooks)

        call_args = mock_run.call_args_list[0][1]
        assert "check" in call_args
        assert call_args["check"] is True

    def test_run_hooks_singular_plural_output(self, capsys):
        """Test correct singular/plural in completion message."""
        # Test singular
        hooks = [Hook(name="single_hook", command=["echo", "test"])]

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks(hooks)

        captured = capsys.readouterr()
        assert "Completed 1/1 hook" in captured.out

        # Test plural
        hooks = [
            Hook(name="hook_1", command=["echo", "test1"]),
            Hook(name="hook_2", command=["echo", "test2"]),
        ]

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks(hooks)

        captured = capsys.readouterr()
        assert "Completed 2/2 hooks" in captured.out

    def test_run_hooks_complex_commands(self):
        """Test hooks with complex command arrays."""
        hooks = [
            Hook(
                name="complex_cmd",
                command=["python", "-c", "print('hello world')"],
            ),
            Hook(name="multiarg_cmd", command=["ls", "-la", "/tmp"]),
        ]

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hooks(hooks)

        # Verify commands are passed through correctly
        first_call = mock_run.call_args_list[0][0][0]
        assert first_call == ["python", "-c", "print('hello world')"]

        second_call = mock_run.call_args_list[1][0][0]
        assert second_call == ["ls", "-la", "/tmp"]


class TestGetHookDict:
    """Test the _get_hook_dict function."""

    def test_get_hook_dict_empty_config(self):
        """Test _get_hook_dict with minimal config (no hooks)."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[Rule(src=Path("/home"), targets=["*.txt"], to="docs")],
        )

        hook_dict = _get_hook_dict(config)
        assert hook_dict == {}

    def test_get_hook_dict_rule_hooks_only(self):
        """Test _get_hook_dict with only rule-level hooks."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[
                Rule(
                    src=Path("/home"),
                    targets=["*.txt"],
                    to="docs",
                    on_start=[Hook(name="start1", command=["echo", "start"])],
                    on_end=[Hook(name="end1", command=["echo", "end"])],
                ),
                Rule(
                    src=Path("/var"),
                    targets=["*.log"],
                    to="logs",
                    on_start=[Hook(name="start2", command=["echo", "start2"])],
                ),
            ],
        )

        hook_dict = _get_hook_dict(config)

        assert len(hook_dict) == 3
        assert "start1" in hook_dict
        assert "end1" in hook_dict
        assert "start2" in hook_dict
        assert hook_dict["start1"].command == ["echo", "start"]
        assert hook_dict["end1"].command == ["echo", "end"]
        assert hook_dict["start2"].command == ["echo", "start2"]

    def test_get_hook_dict_global_hooks_only(self):
        """Test _get_hook_dict with only global hooks."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[Rule(src=Path("/home"), targets=["*.txt"], to="docs")],
            hooks=GlobalHooks(
                pre_backup=[Hook(name="pre_backup", command=["echo", "pre"])],
                post_backup=[Hook(name="post_backup", command=["echo", "post"])],
                pre_restore=[Hook(name="pre_restore", command=["echo", "pre_restore"])],
                post_restore=[
                    Hook(name="post_restore", command=["echo", "post_restore"])
                ],
            ),
        )

        hook_dict = _get_hook_dict(config)

        assert len(hook_dict) == 4
        assert "pre_backup" in hook_dict
        assert "post_backup" in hook_dict
        assert "pre_restore" in hook_dict
        assert "post_restore" in hook_dict

    def test_get_hook_dict_mixed_hooks(self):
        """Test _get_hook_dict with both rule and global hooks."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[
                Rule(
                    src=Path("/home"),
                    targets=["*.txt"],
                    to="docs",
                    on_start=[Hook(name="rule_start", command=["echo", "rule"])],
                )
            ],
            hooks=GlobalHooks(
                pre_backup=[Hook(name="global_pre", command=["echo", "global"])]
            ),
        )

        hook_dict = _get_hook_dict(config)

        assert len(hook_dict) == 2
        assert "rule_start" in hook_dict
        assert "global_pre" in hook_dict

    def test_get_hook_dict_duplicate_names(self):
        """Test _get_hook_dict with duplicate hook names (later overwrites earlier)."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[
                Rule(
                    src=Path("/home"),
                    targets=["*.txt"],
                    to="docs",
                    on_start=[Hook(name="duplicate", command=["echo", "first"])],
                ),
                Rule(
                    src=Path("/var"),
                    targets=["*.log"],
                    to="logs",
                    on_start=[Hook(name="duplicate", command=["echo", "second"])],
                ),
            ],
        )

        hook_dict = _get_hook_dict(config)

        assert len(hook_dict) == 1
        assert "duplicate" in hook_dict
        # Later hook should overwrite earlier one
        assert hook_dict["duplicate"].command == ["echo", "second"]

    def test_get_hook_dict_empty_hook_lists(self):
        """Test _get_hook_dict with empty hook lists."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[
                Rule(
                    src=Path("/home"),
                    targets=["*.txt"],
                    to="docs",
                    on_start=[],  # Empty list
                    on_end=[],  # Empty list
                )
            ],
            hooks=GlobalHooks(
                pre_backup=[], post_backup=[], pre_restore=[], post_restore=[]
            ),
        )

        hook_dict = _get_hook_dict(config)
        assert hook_dict == {}


class TestRunHook:
    """Test the run_hook function (wrapper around run_hooks)."""

    def test_run_hook_success(self, capsys):
        """Test successful single hook execution."""
        hook = Hook(name="test_hook", command=["echo", "test"])

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hook(hook)

        # Should call subprocess.run once
        assert mock_run.call_count == 1

        # Check console output
        captured = capsys.readouterr()
        assert "Running hook (1/1): test_hook" in captured.out
        assert "Completed 1/1 hook" in captured.out

    def test_run_hook_failure(self, capsys):
        """Test single hook execution failure."""
        hook = Hook(name="failing_hook", command=["false"])

        with patch(
            "subprocess.run", side_effect=subprocess.CalledProcessError(1, ["false"])
        ):
            run_hook(hook)

        captured = capsys.readouterr()
        assert "Error executing command `failing_hook`" in captured.out
        assert "Completed 0/1 hook" in captured.out


class TestRunHookByName:
    """Test the run_hook_by_name function."""

    def test_run_hook_by_name_success(self, capsys):
        """Test running a hook by name when it exists."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[
                Rule(
                    src=Path("/home"),
                    targets=["*.txt"],
                    to="docs",
                    on_start=[Hook(name="test_hook", command=["echo", "test"])],
                )
            ],
        )

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hook_by_name(config, "test_hook")

        assert mock_run.call_count == 1
        captured = capsys.readouterr()
        assert "Running hook (1/1): test_hook" in captured.out

    def test_run_hook_by_name_not_found(self, capsys):
        """Test running a hook by name when it doesn't exist."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[Rule(src=Path("/home"), targets=["*.txt"], to="docs")],
        )

        run_hook_by_name(config, "nonexistent_hook")

        captured = capsys.readouterr()
        assert "Hook `nonexistent_hook` not found in the configuration" in captured.out

    def test_run_hook_by_name_multiple_hooks(self):
        """Test running specific hook when multiple exist."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[
                Rule(
                    src=Path("/home"),
                    targets=["*.txt"],
                    to="docs",
                    on_start=[
                        Hook(name="hook1", command=["echo", "first"]),
                        Hook(name="hook2", command=["echo", "second"]),
                    ],
                )
            ],
        )

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hook_by_name(config, "hook2")

        assert mock_run.call_count == 1
        # Verify the correct hook was called
        call_args = mock_run.call_args_list[0][0][0]
        assert call_args == ["echo", "second"]

    def test_run_hook_by_name_from_global_hooks(self):
        """Test running a global hook by name."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[Rule(src=Path("/home"), targets=["*.txt"], to="docs")],
            hooks=GlobalHooks(
                pre_backup=[Hook(name="global_hook", command=["echo", "global"])]
            ),
        )

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock()
            run_hook_by_name(config, "global_hook")

        assert mock_run.call_count == 1
        call_args = mock_run.call_args_list[0][0][0]
        assert call_args == ["echo", "global"]


class TestRunHooksWithInput:
    """Test the run_hooks_with_input function."""

    def test_run_hooks_with_input_no_hooks(self, capsys):
        """Test run_hooks_with_input when no hooks are available."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[Rule(src=Path("/home"), targets=["*.txt"], to="docs")],
        )

        run_hooks_with_input(config)

        captured = capsys.readouterr()
        assert "No hooks defined in the configuration" in captured.out

    def test_run_hooks_with_input_valid_selection(self, capsys):
        """Test run_hooks_with_input with valid hook selection."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[
                Rule(
                    src=Path("/home"),
                    targets=["*.txt"],
                    to="docs",
                    on_start=[
                        Hook(name="hook1", command=["echo", "first"]),
                        Hook(name="hook2", command=["echo", "second"]),
                    ],
                )
            ],
        )

        with patch("click.prompt", return_value="1,2"):
            with patch("subprocess.run") as mock_run:
                mock_run.return_value = MagicMock()
                run_hooks_with_input(config)

        # Should run both hooks
        assert mock_run.call_count == 2

        captured = capsys.readouterr()
        assert "Available hooks:" in captured.out
        assert "[1] hook1" in captured.out
        assert "[2] hook2" in captured.out

    def test_run_hooks_with_input_single_selection(self):
        """Test run_hooks_with_input with single hook selection."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[
                Rule(
                    src=Path("/home"),
                    targets=["*.txt"],
                    to="docs",
                    on_start=[
                        Hook(name="hook1", command=["echo", "first"]),
                        Hook(name="hook2", command=["echo", "second"]),
                    ],
                )
            ],
        )

        with patch("click.prompt", return_value="2"):
            with patch("subprocess.run") as mock_run:
                mock_run.return_value = MagicMock()
                run_hooks_with_input(config)

        # Should run only second hook
        assert mock_run.call_count == 1
        call_args = mock_run.call_args_list[0][0][0]
        assert call_args == ["echo", "second"]

    def test_run_hooks_with_input_invalid_numbers(self):
        """Test run_hooks_with_input with invalid hook numbers."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[
                Rule(
                    src=Path("/home"),
                    targets=["*.txt"],
                    to="docs",
                    on_start=[Hook(name="hook1", command=["echo", "first"])],
                )
            ],
        )

        with patch("click.prompt", return_value="2,99"):  # 2 and 99 are out of range
            with patch("subprocess.run") as mock_run:
                mock_run.return_value = MagicMock()
                run_hooks_with_input(config)

        # Should not run any hooks (all numbers out of range)
        assert mock_run.call_count == 0

    def test_run_hooks_with_input_mixed_valid_invalid(self):
        """Test run_hooks_with_input with mix of valid and invalid selections."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[
                Rule(
                    src=Path("/home"),
                    targets=["*.txt"],
                    to="docs",
                    on_start=[
                        Hook(name="hook1", command=["echo", "first"]),
                        Hook(name="hook2", command=["echo", "second"]),
                    ],
                )
            ],
        )

        with patch("click.prompt", return_value="1,99"):  # 1 valid, 99 invalid
            with patch("subprocess.run") as mock_run:
                mock_run.return_value = MagicMock()
                run_hooks_with_input(config)

        # Should run only the valid hook
        assert mock_run.call_count == 1
        call_args = mock_run.call_args_list[0][0][0]
        assert call_args == ["echo", "first"]

    def test_run_hooks_with_input_empty_selection(self):
        """Test run_hooks_with_input with empty selection."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[
                Rule(
                    src=Path("/home"),
                    targets=["*.txt"],
                    to="docs",
                    on_start=[Hook(name="hook1", command=["echo", "first"])],
                )
            ],
        )

        with patch("click.prompt", return_value=""):
            with patch("subprocess.run") as mock_run:
                mock_run.return_value = MagicMock()
                run_hooks_with_input(config)

        # Should not run any hooks
        assert mock_run.call_count == 0

    def test_run_hooks_with_input_whitespace_handling(self):
        """Test run_hooks_with_input handles whitespace in selection."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[
                Rule(
                    src=Path("/home"),
                    targets=["*.txt"],
                    to="docs",
                    on_start=[
                        Hook(name="hook1", command=["echo", "first"]),
                        Hook(name="hook2", command=["echo", "second"]),
                    ],
                )
            ],
        )

        with patch("click.prompt", return_value=" 1 , 2 "):  # Extra whitespace
            with patch("subprocess.run") as mock_run:
                mock_run.return_value = MagicMock()
                run_hooks_with_input(config)

        # Should still run both hooks
        assert mock_run.call_count == 2

    def test_run_hooks_with_input_invalid_input_exception(self, capsys):
        """Test run_hooks_with_input with invalid input that causes exception."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[
                Rule(
                    src=Path("/home"),
                    targets=["*.txt"],
                    to="docs",
                    on_start=[Hook(name="hook1", command=["echo", "first"])],
                )
            ],
        )

        with patch("click.prompt", return_value="abc"):  # Non-numeric input
            run_hooks_with_input(config)

        captured = capsys.readouterr()
        assert "Input invalid:" in captured.out

    def test_run_hooks_with_input_displays_all_hooks(self, capsys):
        """Test that run_hooks_with_input displays hooks from all sources."""
        config = Config(
            destination=Path("/tmp/backup"),
            rules=[
                Rule(
                    src=Path("/home"),
                    targets=["*.txt"],
                    to="docs",
                    on_start=[Hook(name="rule_hook", command=["echo", "rule"])],
                )
            ],
            hooks=GlobalHooks(
                pre_backup=[Hook(name="global_hook", command=["echo", "global"])]
            ),
        )

        with patch("click.prompt", return_value=""):  # Don't actually run anything
            run_hooks_with_input(config)

        captured = capsys.readouterr()
        assert "Available hooks:" in captured.out
        assert "[1] rule_hook" in captured.out
        assert "[2] global_hook" in captured.out
