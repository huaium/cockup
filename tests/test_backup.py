import tempfile
from pathlib import Path
from unittest.mock import patch

from cockup.src.backup import backup
from cockup.src.config import Config, Hooks, Rule


class TestBackup:
    """Test the backup function."""

    def test_backup_clean_mode_existing_directory(self, capsys):
        """Test backup with clean mode and existing directory."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"
            destination.mkdir()
            (destination / "old_file.txt").write_text("old content")

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            hooks = Hooks(
                pre_backup=[], post_backup=[], pre_restore=[], post_restore=[]
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=True,
                metadata=True,
            )

            with patch("shutil.rmtree") as mock_rmtree:
                with patch("cockup.src.backup.handle_rules") as mock_handle_rules:
                    with patch("os.chdir") as mock_chdir:
                        backup(config)

            # Verify clean mode behavior
            mock_rmtree.assert_called_once_with(destination)
            assert destination.exists()  # Should be recreated
            mock_chdir.assert_called_once_with(destination)
            mock_handle_rules.assert_called_once_with([rule], True, "backup")

            captured = capsys.readouterr()
            assert "Clean mode enabled" in captured.out
            assert "Found existing backup folder, removing..." in captured.out

    def test_backup_clean_mode_no_existing_directory(self, capsys):
        """Test backup with clean mode but no existing directory."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "nonexistent_backup"

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            hooks = Hooks(
                pre_backup=[], post_backup=[], pre_restore=[], post_restore=[]
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=True,
                metadata=True,
            )

            with patch("shutil.rmtree") as mock_rmtree:
                with patch("cockup.src.backup.handle_rules") as mock_handle_rules:
                    with patch("os.chdir") as mock_chdir:
                        backup(config)

            # rmtree should not be called since directory doesn't exist
            mock_rmtree.assert_not_called()
            assert destination.exists()  # Should be created
            mock_chdir.assert_called_once_with(destination)
            mock_handle_rules.assert_called_once_with([rule], True, "backup")

            captured = capsys.readouterr()
            assert "Clean mode enabled" in captured.out
            assert (
                "Existing backup folder not found, creating a new one" in captured.out
            )

    def test_backup_no_clean_mode(self, capsys):
        """Test backup without clean mode."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            hooks = Hooks(
                pre_backup=[], post_backup=[], pre_restore=[], post_restore=[]
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=False,
            )

            with patch("shutil.rmtree") as mock_rmtree:
                with patch("cockup.src.backup.handle_rules") as mock_handle_rules:
                    with patch("os.chdir") as mock_chdir:
                        backup(config)

            # rmtree should not be called
            mock_rmtree.assert_not_called()
            assert destination.exists()  # Should be created
            mock_chdir.assert_called_once_with(destination)
            mock_handle_rules.assert_called_once_with([rule], False, "backup")

            captured = capsys.readouterr()
            assert "Clean mode disabled" in captured.out
            assert "will not remove existing backup folder" in captured.out

    def test_backup_with_pre_hooks(self, capsys):
        """Test backup with pre-backup hooks."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            pre_backup_hooks = [{"name": "pre_backup", "command": ["echo", "before"]}]
            hooks = Hooks(
                pre_backup=pre_backup_hooks,
                post_backup=[],
                pre_restore=[],
                post_restore=[],
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            with patch("cockup.src.backup.run_hooks") as mock_run_hooks:
                with patch("cockup.src.backup.handle_rules") as _:
                    with patch("os.chdir"):
                        backup(config)

            # Verify pre-backup hooks were called
            mock_run_hooks.assert_called_once_with(pre_backup_hooks)

            captured = capsys.readouterr()
            assert "Running pre-backup hooks..." in captured.out

    def test_backup_with_post_hooks(self, capsys):
        """Test backup with post-backup hooks."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            post_backup_hooks = [{"name": "post_backup", "command": ["echo", "after"]}]
            hooks = Hooks(
                pre_backup=[],
                post_backup=post_backup_hooks,
                pre_restore=[],
                post_restore=[],
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            with patch("cockup.src.backup.run_hooks") as mock_run_hooks:
                with patch("cockup.src.backup.handle_rules") as _:
                    with patch("os.chdir"):
                        backup(config)

            # Verify post-backup hooks were called
            mock_run_hooks.assert_called_once_with(post_backup_hooks)

            captured = capsys.readouterr()
            assert "Running post-backup hooks..." in captured.out

    def test_backup_with_both_hooks(self, capsys):
        """Test backup with both pre and post hooks."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            pre_backup_hooks = [{"name": "pre", "command": ["echo", "before"]}]
            post_backup_hooks = [{"name": "post", "command": ["echo", "after"]}]
            hooks = Hooks(
                pre_backup=pre_backup_hooks,
                post_backup=post_backup_hooks,
                pre_restore=[],
                post_restore=[],
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            with patch("cockup.src.backup.run_hooks") as mock_run_hooks:
                with patch("cockup.src.backup.handle_rules") as _:
                    with patch("os.chdir"):
                        backup(config)

            # Verify both hooks were called
            assert mock_run_hooks.call_count == 2
            mock_run_hooks.assert_any_call(pre_backup_hooks)
            mock_run_hooks.assert_any_call(post_backup_hooks)

            captured = capsys.readouterr()
            assert "Running pre-backup hooks..." in captured.out
            assert "Running post-backup hooks..." in captured.out

    def test_backup_no_hooks(self):
        """Test backup with no hooks."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            hooks = Hooks(
                pre_backup=[], post_backup=[], pre_restore=[], post_restore=[]
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            with patch("cockup.src.backup.run_hooks") as mock_run_hooks:
                with patch("cockup.src.backup.handle_rules") as _:
                    with patch("os.chdir"):
                        backup(config)

            # No hooks should be called
            mock_run_hooks.assert_not_called()

    def test_backup_directory_creation_nested(self):
        """Test backup creates nested directories."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "deep" / "nested" / "backup"

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            hooks = Hooks(
                pre_backup=[], post_backup=[], pre_restore=[], post_restore=[]
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            with patch("cockup.src.backup.handle_rules") as _:
                with patch("os.chdir"):
                    backup(config)

            # Verify nested directory was created
            assert destination.exists()
            assert destination.is_dir()

    def test_backup_workflow_order(self, capsys):
        """Test that backup workflow follows correct order."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            hooks = Hooks(
                pre_backup=[{"name": "pre", "command": ["echo", "pre"]}],
                post_backup=[{"name": "post", "command": ["echo", "post"]}],
                pre_restore=[],
                post_restore=[],
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
                if hook_list == hooks.pre_backup:
                    call_order.append("pre_hooks")
                elif hook_list == hooks.post_backup:
                    call_order.append("post_hooks")

            def mock_handle_rules(*args):
                call_order.append("handle_rules")

            with patch("cockup.src.backup.run_hooks", side_effect=mock_run_hooks):
                with patch(
                    "cockup.src.backup.handle_rules", side_effect=mock_handle_rules
                ):
                    with patch("os.chdir"):
                        backup(config)

            # Verify order: pre_hooks -> handle_rules -> post_hooks
            assert call_order == ["pre_hooks", "handle_rules", "post_hooks"]

            captured = capsys.readouterr()
            assert captured.out.index("Starting backup...") < captured.out.index(
                "Running pre-backup hooks..."
            )
            assert captured.out.index(
                "Running pre-backup hooks..."
            ) < captured.out.index("Running post-backup hooks...")
            assert captured.out.index(
                "Running post-backup hooks..."
            ) < captured.out.index("Backup completed.")

    def test_backup_multiple_rules(self):
        """Test backup with multiple rules."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"

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
                pre_backup=[], post_backup=[], pre_restore=[], post_restore=[]
            )
            config = Config(
                destination=destination,
                rules=rules,
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            with patch("cockup.src.backup.handle_rules") as mock_handle_rules:
                with patch("os.chdir"):
                    backup(config)

            # Verify handle_rules called with all rules
            mock_handle_rules.assert_called_once_with(rules, True, "backup")

    def test_backup_console_output_messages(self, capsys):
        """Test all expected console output messages."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            destination = Path(tmp_dir) / "backup"

            rule = Rule(
                src=Path(tmp_dir) / "src",
                targets=["*.txt"],
                to="docs",
                on_start=[],
                on_end=[],
            )
            hooks = Hooks(
                pre_backup=[], post_backup=[], pre_restore=[], post_restore=[]
            )
            config = Config(
                destination=destination,
                rules=[rule],
                hooks=hooks,
                clean=False,
                metadata=True,
            )

            with patch("cockup.src.backup.handle_rules"):
                with patch("os.chdir"):
                    backup(config)

            captured = capsys.readouterr()
            expected_messages = [
                "Starting backup...",
                "Clean mode disabled",
                "Backup completed.",
            ]

            for message in expected_messages:
                assert message in captured.out
