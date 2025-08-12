import stat
import tempfile
from pathlib import Path
from unittest.mock import MagicMock, patch

from cockup.src.config import Rule
from cockup.src.rules import _abbreviate_home, _handle_rule, _smart_copy, handle_rules


class TestAbbreviateHome:
    """Test the _abbreviate_home function."""

    def test_abbreviate_home_success(self):
        """Test successful home directory abbreviation."""
        home_path = Path.home() / "Documents" / "test.txt"
        result = _abbreviate_home(home_path)
        assert result == "~/Documents/test.txt"

    def test_abbreviate_home_not_in_home(self):
        """Test path not in home directory."""
        root_path = Path("/etc/passwd")
        result = _abbreviate_home(root_path)
        assert result == str(root_path)

    def test_abbreviate_home_symlink(self):
        """Test path with symlink handling."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            # Create a test path that simulates home
            test_path = Path(tmp_dir) / "test_file.txt"
            test_path.write_text("test")

            # Mock Path.home() to return our temp directory
            with patch("pathlib.Path.home", return_value=Path(tmp_dir)):
                result = _abbreviate_home(test_path)
                assert result == "~/test_file.txt"

    def test_abbreviate_home_relative_path_exception(self):
        """Test exception handling in abbreviate_home."""
        # Create a path that will cause relative_to to raise an exception
        with tempfile.TemporaryDirectory() as tmp_dir:
            test_path = Path(tmp_dir) / "test.txt"
            test_path.write_text("test")

            # Mock Path.home() to return a different path
            with patch("pathlib.Path.home", return_value=Path("/different/path")):
                result = _abbreviate_home(test_path)
                # Should return absolute path when not in home
                assert str(test_path.absolute()) in result


class TestSmartCopy:
    """Test the _smart_copy function."""

    def test_smart_copy_file_new(self, capsys):
        """Test copying a new file."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            src_file = Path(tmp_dir) / "src.txt"
            dst_file = Path(tmp_dir) / "dst.txt"
            src_file.write_text("test content")

            _smart_copy(src_file, dst_file, metadata=True)

            assert dst_file.exists()
            assert dst_file.read_text() == "test content"

            captured = capsys.readouterr()
            assert "File copied:" in captured.out

    def test_smart_copy_file_update(self, capsys):
        """Test updating an existing file."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            src_file = Path(tmp_dir) / "src.txt"
            dst_file = Path(tmp_dir) / "dst.txt"
            src_file.write_text("new content")
            dst_file.write_text("old content")

            _smart_copy(src_file, dst_file, metadata=True)

            assert dst_file.read_text() == "new content"

            captured = capsys.readouterr()
            assert "File existed, updating:" in captured.out

    def test_smart_copy_symlink(self, capsys):
        """Test copying a symlink."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            target_file = Path(tmp_dir) / "target.txt"
            src_link = Path(tmp_dir) / "src_link"
            dst_link = Path(tmp_dir) / "dst_link"

            target_file.write_text("link target")
            src_link.symlink_to(target_file)

            _smart_copy(src_link, dst_link, metadata=True)

            assert dst_link.is_symlink()
            assert dst_link.readlink() == target_file

            captured = capsys.readouterr()
            assert "Symlink copied:" in captured.out

    def test_smart_copy_directory_new(self, capsys):
        """Test copying a new directory."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            src_dir = Path(tmp_dir) / "src_dir"
            dst_dir = Path(tmp_dir) / "dst_dir"
            src_dir.mkdir()
            (src_dir / "file.txt").write_text("content")

            _smart_copy(src_dir, dst_dir, metadata=True)

            assert dst_dir.is_dir()
            assert (dst_dir / "file.txt").read_text() == "content"

            captured = capsys.readouterr()
            assert "Folder copied:" in captured.out

    def test_smart_copy_directory_update(self, capsys):
        """Test updating an existing directory."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            src_dir = Path(tmp_dir) / "src_dir"
            dst_dir = Path(tmp_dir) / "dst_dir"
            src_dir.mkdir()
            dst_dir.mkdir()
            (src_dir / "new_file.txt").write_text("new content")
            (dst_dir / "old_file.txt").write_text("old content")

            with patch("shutil.rmtree") as mock_rmtree:
                _smart_copy(src_dir, dst_dir, metadata=True)

            mock_rmtree.assert_called_once_with(dst_dir)
            assert dst_dir.is_dir()
            assert (dst_dir / "new_file.txt").exists()

            captured = capsys.readouterr()
            assert "Folder existed, updating:" in captured.out

    def test_smart_copy_metadata_flags(self):
        """Test metadata preservation flags."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            src_file = Path(tmp_dir) / "src.txt"
            dst_file1 = Path(tmp_dir) / "dst1.txt"
            dst_file2 = Path(tmp_dir) / "dst2.txt"
            src_file.write_text("test")

            with patch("shutil.copy2") as mock_copy2:
                with patch("shutil.copy") as mock_copy:
                    # Test with metadata=True
                    _smart_copy(src_file, dst_file1, metadata=True)
                    mock_copy2.assert_called()
                    mock_copy.assert_not_called()

            with patch("shutil.copy2") as mock_copy2:
                with patch("shutil.copy") as mock_copy:
                    # Test with metadata=False
                    _smart_copy(src_file, dst_file2, metadata=False)
                    mock_copy.assert_called()
                    mock_copy2.assert_not_called()

    def test_smart_copy_no_progress_output(self):
        """Test _smart_copy with print_progress=False."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            src_file = Path(tmp_dir) / "src.txt"
            dst_file = Path(tmp_dir) / "dst.txt"
            src_file.write_text("test content")

            with patch("cockup.src.rules.rprint") as mock_rprint:
                _smart_copy(src_file, dst_file, metadata=True, print_progress=False)

            # rprint should not be called when print_progress=False
            mock_rprint.assert_not_called()

    def test_smart_copy_special_file_handling(self, capsys):
        """Test handling of special files (non-regular)."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            src_path = Path(tmp_dir) / "special"
            dst_path = Path(tmp_dir) / "dst_special"

            # Mock os.lstat to return a socket file mode
            mock_stat_result = MagicMock()
            mock_stat_result.st_mode = stat.S_IFSOCK | 0o644  # Socket file

            with patch("os.lstat", return_value=mock_stat_result):
                _smart_copy(src_path, dst_path, metadata=True)

            captured = capsys.readouterr()
            assert "Skipping non-regular file:" in captured.out

    def test_smart_copy_error_handling(self, capsys):
        """Test error handling in _smart_copy."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            src_file = Path(tmp_dir) / "nonexistent.txt"
            dst_file = Path(tmp_dir) / "dst.txt"

            _smart_copy(src_file, dst_file, metadata=True)

            captured = capsys.readouterr()
            assert "Error copying:" in captured.out


class TestHandleRule:
    """Test the _handle_rule function."""

    def test_handle_rule_backup_existing_file(self):
        """Test handling rule for backup with existing file."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            src_dir = Path(tmp_dir) / "src"
            src_dir.mkdir()
            (src_dir / "test.txt").write_text("content")

            rule = Rule(
                src=src_dir, targets=["test.txt"], to="backup", on_start=[], on_end=[]
            )

            with patch("pathlib.Path.cwd", return_value=Path(tmp_dir)):
                with patch("cockup.src.rules._smart_copy") as mock_smart_copy:
                    _handle_rule(rule, metadata=True, direction="backup")

                mock_smart_copy.assert_called_once()
                # Verify source and destination paths
                src_arg, dst_arg = (
                    mock_smart_copy.call_args[1]["src"],
                    mock_smart_copy.call_args[1]["dst"],
                )
                assert src_arg.name == "test.txt"
                assert "backup" in str(dst_arg)

    def test_handle_rule_restore_existing_file(self):
        """Test handling rule for restore with existing file."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            src_dir = Path(tmp_dir) / "src"
            backup_dir = Path(tmp_dir) / "backup"
            src_dir.mkdir()
            backup_dir.mkdir()
            (backup_dir / "test.txt").write_text("backup content")

            rule = Rule(
                src=src_dir, targets=["test.txt"], to="backup", on_start=[], on_end=[]
            )

            with patch("pathlib.Path.cwd", return_value=Path(tmp_dir)):
                with patch("cockup.src.rules._smart_copy") as mock_smart_copy:
                    _handle_rule(rule, metadata=True, direction="restore")

                mock_smart_copy.assert_called_once()

    def test_handle_rule_glob_patterns(self, capsys):
        """Test handling rule with glob patterns."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            src_dir = Path(tmp_dir) / "src"
            src_dir.mkdir()
            (src_dir / "file1.txt").write_text("content1")
            (src_dir / "file2.txt").write_text("content2")
            (src_dir / "file.pdf").write_text("pdf content")

            rule = Rule(
                src=src_dir, targets=["*.txt"], to="backup", on_start=[], on_end=[]
            )

            with patch("pathlib.Path.cwd", return_value=Path(tmp_dir)):
                with patch("glob.has_magic", return_value=True):
                    with patch(
                        "glob.glob",
                        return_value=[
                            str(src_dir / "file1.txt"),
                            str(src_dir / "file2.txt"),
                        ],
                    ):
                        with patch("cockup.src.rules._smart_copy") as mock_smart_copy:
                            _handle_rule(rule, metadata=True, direction="backup")

                # Should be called twice for the two matched files
                assert mock_smart_copy.call_count == 2

                captured = capsys.readouterr()
                assert "Pattern matched (2 founded):" in captured.out

    def test_handle_rule_glob_no_matches(self, capsys):
        """Test handling rule with glob pattern that matches nothing."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            src_dir = Path(tmp_dir) / "src"
            src_dir.mkdir()

            rule = Rule(
                src=src_dir,
                targets=["*.nonexistent"],
                to="backup",
                on_start=[],
                on_end=[],
            )

            with patch("pathlib.Path.cwd", return_value=Path(tmp_dir)):
                with patch("glob.has_magic", return_value=True):
                    with patch("glob.glob", return_value=[]):
                        _handle_rule(rule, metadata=True, direction="backup")

                captured = capsys.readouterr()
                assert "Matches not found for pattern:" in captured.out

    def test_handle_rule_missing_source(self, capsys):
        """Test handling rule with missing source file."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            src_dir = Path(tmp_dir) / "src"
            src_dir.mkdir()

            rule = Rule(
                src=src_dir,
                targets=["nonexistent.txt"],
                to="backup",
                on_start=[],
                on_end=[],
            )

            with patch("pathlib.Path.cwd", return_value=Path(tmp_dir)):
                _handle_rule(rule, metadata=True, direction="backup")

                captured = capsys.readouterr()
                assert "Source not found, skipping:" in captured.out

    def test_handle_rule_multiple_targets(self):
        """Test handling rule with multiple targets."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            src_dir = Path(tmp_dir) / "src"
            src_dir.mkdir()
            (src_dir / "file1.txt").write_text("content1")
            (src_dir / "file2.pdf").write_text("content2")

            rule = Rule(
                src=src_dir,
                targets=["file1.txt", "file2.pdf"],
                to="backup",
                on_start=[],
                on_end=[],
            )

            with patch("pathlib.Path.cwd", return_value=Path(tmp_dir)):
                with patch("cockup.src.rules._smart_copy") as mock_smart_copy:
                    _handle_rule(rule, metadata=True, direction="backup")

                # Should be called twice for both targets
                assert mock_smart_copy.call_count == 2


class TestHandleRules:
    """Test the handle_rules function."""

    def test_handle_rules_basic(self, capsys):
        """Test basic handle_rules functionality."""
        with tempfile.TemporaryDirectory() as tmp_dir:
            src_dir = Path(tmp_dir) / "src"
            src_dir.mkdir()

            rules = [
                Rule(
                    src=src_dir, targets=["*.txt"], to="backup", on_start=[], on_end=[]
                )
            ]

            with patch("cockup.src.rules._handle_rule") as mock_handle_rule:
                handle_rules(rules, metadata=True, direction="backup")

                mock_handle_rule.assert_called_once()
                captured = capsys.readouterr()
                assert "Metadata preservation enabled" in captured.out

    def test_handle_rules_metadata_disabled(self, capsys):
        """Test handle_rules with metadata disabled."""
        rules = [
            Rule(
                src=Path("/tmp/src"),
                targets=["*.txt"],
                to="backup",
                on_start=[],
                on_end=[],
            )
        ]

        with patch("cockup.src.rules._handle_rule"):
            handle_rules(rules, metadata=False, direction="backup")

            captured = capsys.readouterr()
            assert "Metadata preservation disabled" in captured.out

    def test_handle_rules_with_hooks(self, capsys):
        """Test handle_rules with rule-specific hooks."""
        rules = [
            Rule(
                src=Path("/tmp/src"),
                targets=["*.txt"],
                to="backup",
                on_start=[{"name": "pre", "command": ["echo", "before"]}],
                on_end=[{"name": "post", "command": ["echo", "after"]}],
            )
        ]

        with patch("cockup.src.rules._handle_rule"):
            with patch("cockup.src.rules.run_hooks") as mock_run_hooks:
                handle_rules(rules, metadata=True, direction="backup")

                # Should call hooks twice (on_start and on_end)
                assert mock_run_hooks.call_count == 2

                captured = capsys.readouterr()
                assert "Running pre-rule hooks for Rule 1..." in captured.out
                assert "Running post-rule hooks for Rule 1..." in captured.out

    def test_handle_rules_multiple_rules_with_hooks(self, capsys):
        """Test handle_rules with multiple rules having hooks."""
        rules = [
            Rule(
                src=Path("/tmp/src1"),
                targets=["*.txt"],
                to="backup1",
                on_start=[{"name": "pre1", "command": ["echo", "before1"]}],
                on_end=[],
            ),
            Rule(
                src=Path("/tmp/src2"),
                targets=["*.pdf"],
                to="backup2",
                on_start=[],
                on_end=[{"name": "post2", "command": ["echo", "after2"]}],
            ),
        ]

        with patch("cockup.src.rules._handle_rule"):
            with patch("cockup.src.rules.run_hooks") as mock_run_hooks:
                handle_rules(rules, metadata=True, direction="backup")

                # Should call hooks twice (one for each rule)
                assert mock_run_hooks.call_count == 2

                captured = capsys.readouterr()
                assert "Running pre-rule hooks for Rule 1..." in captured.out
                assert "Running post-rule hooks for Rule 2..." in captured.out

    def test_handle_rules_no_hooks(self):
        """Test handle_rules with no rule-specific hooks."""
        rules = [
            Rule(
                src=Path("/tmp/src"),
                targets=["*.txt"],
                to="backup",
                on_start=[],
                on_end=[],
            )
        ]

        with patch("cockup.src.rules._handle_rule"):
            with patch("cockup.src.rules.run_hooks") as mock_run_hooks:
                handle_rules(rules, metadata=True, direction="backup")

                # No hooks should be called
                mock_run_hooks.assert_not_called()

    def test_handle_rules_direction_parameter(self):
        """Test that direction parameter is passed correctly."""
        rules = [
            Rule(
                src=Path("/tmp/src"),
                targets=["*.txt"],
                to="backup",
                on_start=[],
                on_end=[],
            )
        ]

        with patch("cockup.src.rules._handle_rule") as mock_handle_rule:
            # Test backup direction
            handle_rules(rules, metadata=True, direction="backup")
            mock_handle_rule.assert_called_with(rules[0], True, "backup")

            # Test restore direction
            handle_rules(rules, metadata=True, direction="restore")
            mock_handle_rule.assert_called_with(rules[0], True, "restore")

    def test_handle_rules_empty_rules_list(self, capsys):
        """Test handle_rules with empty rules list."""
        with patch("cockup.src.rules._handle_rule") as mock_handle_rule:
            handle_rules([], metadata=True, direction="backup")

            mock_handle_rule.assert_not_called()
            captured = capsys.readouterr()
            assert "Metadata preservation enabled" in captured.out
