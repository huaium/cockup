import tempfile
from pathlib import Path
from unittest.mock import patch

from cockup.src.config import Config, Hooks, Rule
from cockup.src.restore import restore


class TestRestore:
    """Test the restore function."""

    def test_restore_basic(self, capsys):
        """Test basic restore functionality."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"
            destination.mkdir()

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            hooks = Hooks(
                pre_restore=[], post_restore=[], pre_backup=[], post_backup=[]
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            with patch("cockup.src.restore.handle_rules") as mock_handle_rules:
                with patch("os.chdir") as mock_chdir:
                    restore(config)

            # Verify restore behavior
            mock_chdir.assert_called_once_with(destination)
            mock_handle_rules.assert_called_once_with([rule], True, "restore")

            captured = capsys.readouterr()
            assert "Starting restore..." in captured.out
            assert "Restore completed." in captured.out

    def test_restore_with_pre_hooks(self, capsys):
        """Test restore with pre-restore hooks."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"
            destination.mkdir()

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            pre_restore_hooks = [{"name": "pre_restore", "command": ["echo", "before"]}]
            hooks = Hooks(
                pre_restore=pre_restore_hooks,
                post_restore=[],
                pre_backup=[],
                post_backup=[],
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            with patch("cockup.src.restore.run_hooks") as mock_run_hooks:
                with patch("cockup.src.restore.handle_rules"):
                    with patch("os.chdir"):
                        restore(config)

            # Verify pre-restore hooks were called
            mock_run_hooks.assert_called_once_with(pre_restore_hooks)

            captured = capsys.readouterr()
            assert "Running pre-restore hooks..." in captured.out

    def test_restore_with_post_hooks(self, capsys):
        """Test restore with post-restore hooks."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"
            destination.mkdir()

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            post_restore_hooks = [
                {"name": "post_restore", "command": ["echo", "after"]}
            ]
            hooks = Hooks(
                pre_restore=[],
                post_restore=post_restore_hooks,
                pre_backup=[],
                post_backup=[],
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            with patch("cockup.src.restore.run_hooks") as mock_run_hooks:
                with patch("cockup.src.restore.handle_rules"):
                    with patch("os.chdir"):
                        restore(config)

            # Verify post-restore hooks were called
            mock_run_hooks.assert_called_once_with(post_restore_hooks)

            captured = capsys.readouterr()
            assert "Running post-restore hooks..." in captured.out

    def test_restore_with_both_hooks(self, capsys):
        """Test restore with both pre and post hooks."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"
            destination.mkdir()

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            pre_restore_hooks = [{"name": "pre", "command": ["echo", "before"]}]
            post_restore_hooks = [{"name": "post", "command": ["echo", "after"]}]
            hooks = Hooks(
                pre_restore=pre_restore_hooks,
                post_restore=post_restore_hooks,
                pre_backup=[],
                post_backup=[],
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            with patch("cockup.src.restore.run_hooks") as mock_run_hooks:
                with patch("cockup.src.restore.handle_rules"):
                    with patch("os.chdir"):
                        restore(config)

            # Verify both hooks were called
            assert mock_run_hooks.call_count == 2
            mock_run_hooks.assert_any_call(pre_restore_hooks)
            mock_run_hooks.assert_any_call(post_restore_hooks)

            captured = capsys.readouterr()
            assert "Running pre-restore hooks..." in captured.out
            assert "Running post-restore hooks..." in captured.out

    def test_restore_no_hooks(self):
        """Test restore with no hooks."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"
            destination.mkdir()

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            hooks = Hooks(
                pre_restore=[], post_restore=[], pre_backup=[], post_backup=[]
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            with patch("cockup.src.restore.run_hooks") as mock_run_hooks:
                with patch("cockup.src.restore.handle_rules"):
                    with patch("os.chdir"):
                        restore(config)

            # No hooks should be called
            mock_run_hooks.assert_not_called()

    def test_restore_workflow_order(self, capsys):
        """Test that restore workflow follows correct order."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"
            destination.mkdir()

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            hooks = Hooks(
                pre_restore=[{"name": "pre", "command": ["echo", "pre"]}],
                post_restore=[{"name": "post", "command": ["echo", "post"]}],
                pre_backup=[],
                post_backup=[],
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            call_order = []

            def mock_run_hooks(hook_list):
                if hook_list == hooks.pre_restore:
                    call_order.append("pre_hooks")
                elif hook_list == hooks.post_restore:
                    call_order.append("post_hooks")

            def mock_handle_rules(*args):
                call_order.append("handle_rules")

            with patch("cockup.src.restore.run_hooks", side_effect=mock_run_hooks):
                with patch(
                    "cockup.src.restore.handle_rules", side_effect=mock_handle_rules
                ):
                    with patch("os.chdir"):
                        restore(config)

            # Verify order: pre_hooks -> handle_rules -> post_hooks
            assert call_order == ["pre_hooks", "handle_rules", "post_hooks"]

            captured = capsys.readouterr()
            assert captured.out.index("Starting restore...") < captured.out.index(
                "Running pre-restore hooks..."
            )
            assert captured.out.index(
                "Running pre-restore hooks..."
            ) < captured.out.index("Running post-restore hooks...")
            assert captured.out.index(
                "Running post-restore hooks..."
            ) < captured.out.index("Restore completed.")

    def test_restore_multiple_rules(self):
        """Test restore with multiple rules."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"
            destination.mkdir()

            rules = [
                Rule(
                    src=Path(tmp_dir) / "src1",
                    targets=["*.txt"],
                    to="docs",
                    on_start=[],
                    on_end=[],
                ),
                Rule(
                    src=Path(tmp_dir) / "src2",
                    targets=["*.pdf"],
                    to="pdfs",
                    on_start=[],
                    on_end=[],
                ),
            ]
            hooks = Hooks(
                pre_restore=[], post_restore=[], pre_backup=[], post_backup=[]
            )
            config = Config(
                destination=destination,
                rules=rules,
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            with patch("cockup.src.restore.handle_rules") as mock_handle_rules:
                with patch("os.chdir"):
                    restore(config)

            # Verify handle_rules called with all rules
            mock_handle_rules.assert_called_once_with(rules, True, "restore")

    def test_restore_metadata_flags(self):
        """Test restore passes correct metadata flag."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"
            destination.mkdir()

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            hooks = Hooks(
                pre_restore=[], post_restore=[], pre_backup=[], post_backup=[]
            )

            # Test with metadata=True
            config_true = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            with patch("cockup.src.restore.handle_rules") as mock_handle_rules:
                with patch("os.chdir"):
                    restore(config_true)

            mock_handle_rules.assert_called_once_with([rule], True, "restore")

            # Test with metadata=False
            config_false = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=False,
            )

            with patch("cockup.src.restore.handle_rules") as mock_handle_rules:
                with patch("os.chdir"):
                    restore(config_false)

            mock_handle_rules.assert_called_with([rule], False, "restore")

    def test_restore_directory_change(self):
        """Test that restore changes to correct directory."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"
            destination.mkdir()

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            hooks = Hooks(
                pre_restore=[], post_restore=[], pre_backup=[], post_backup=[]
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            with patch("cockup.src.restore.handle_rules"):
                with patch("os.chdir") as mock_chdir:
                    restore(config)

            # Verify directory change
            mock_chdir.assert_called_once_with(destination)

    def test_restore_console_output_messages(self, capsys):
        """Test all expected console output messages."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"
            destination.mkdir()

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            hooks = Hooks(
                pre_restore=[], post_restore=[], pre_backup=[], post_backup=[]
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            with patch("cockup.src.restore.handle_rules"):
                with patch("os.chdir"):
                    restore(config)

            captured = capsys.readouterr()
            expected_messages = ["Starting restore...", "Restore completed."]

            for message in expected_messages:
                assert message in captured.out

    def test_restore_ignores_clean_flag(self):
        """Test that restore ignores the clean flag (it's only for backup)."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"
            destination.mkdir()

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            hooks = Hooks(
                pre_restore=[], post_restore=[], pre_backup=[], post_backup=[]
            )

            # Test with clean=True (should have no effect on restore)
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=True,  # This should be ignored for restore
                metadata=True,
            )

            with patch("cockup.src.restore.handle_rules") as mock_handle_rules:
                with patch("os.chdir"):
                    restore(config)

            # Verify restore still works normally
            mock_handle_rules.assert_called_once_with([rule], True, "restore")

    def test_restore_different_hook_types_ignored(self):
        """Test that restore ignores backup hooks."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"
            destination.mkdir()

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            hooks = Hooks(
                pre_restore=[
                    {"name": "pre_restore", "command": ["echo", "pre_restore"]}
                ],
                post_restore=[
                    {"name": "post_restore", "command": ["echo", "post_restore"]}
                ],
                pre_backup=[{"name": "pre_backup", "command": ["echo", "pre_backup"]}],
                post_backup=[
                    {"name": "post_backup", "command": ["echo", "post_backup"]}
                ],
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            called_hooks = []

            def mock_run_hooks(hook_list):
                called_hooks.extend(hook_list)

            with patch("cockup.src.restore.run_hooks", side_effect=mock_run_hooks):
                with patch("cockup.src.restore.handle_rules"):
                    with patch("os.chdir"):
                        restore(config)

            # Only restore hooks should have been called
            assert len(called_hooks) == 2
            assert called_hooks[0]["name"] == "pre_restore"
            assert called_hooks[1]["name"] == "post_restore"
