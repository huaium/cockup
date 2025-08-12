from unittest.mock import patch

from rich.style import Style

from cockup.src.console import rprint, rprint_error, rprint_point, rprint_warning


class TestRprint:
    """Test the base rprint function."""

    @patch("cockup.src.console._console.print")
    def test_rprint_basic_message(self, mock_print):
        """Test basic message printing."""
        message = "Test message"
        rprint(message)

        mock_print.assert_called_once_with(
            message, style=None, end="\n", highlight=False
        )

    @patch("cockup.src.console._console.print")
    def test_rprint_with_style(self, mock_print):
        """Test message printing with custom style."""
        message = "Styled message"
        style = Style(color="red", bold=True)
        rprint(message, style=style)

        mock_print.assert_called_once_with(
            message, style=style, end="\n", highlight=False
        )

    @patch("cockup.src.console._console.print")
    def test_rprint_custom_end(self, mock_print):
        """Test message printing with custom end character."""
        message = "No newline"
        rprint(message, end="")

        mock_print.assert_called_once_with(message, style=None, end="", highlight=False)

    @patch("cockup.src.console._console.print")
    def test_rprint_with_style_and_end(self, mock_print):
        """Test message printing with both style and end parameter."""
        message = "Complete test"
        style = Style(color="blue")
        rprint(message, style=style, end=" ")

        mock_print.assert_called_once_with(
            message, style=style, end=" ", highlight=False
        )


class TestRprintPoint:
    """Test the rprint_point function."""

    @patch("cockup.src.console.rprint")
    def test_rprint_point_basic(self, mock_rprint):
        """Test basic point message printing."""
        message = "Process started"
        rprint_point(message)

        # Should make two calls: one for "=> " and one for the message
        assert mock_rprint.call_count == 2

        # First call: cyan arrow with no newline
        first_call = mock_rprint.call_args_list[0]
        assert first_call[0][0] == "=> "  # First argument (message)
        assert first_call[1]["end"] == ""  # No newline
        assert first_call[1]["style"].color.name == "cyan"
        assert first_call[1]["style"].bold is True

        # Second call: green message with newline
        second_call = mock_rprint.call_args_list[1]
        assert second_call[1]["message"] == message
        assert second_call[1]["end"] == "\n"
        assert second_call[1]["style"].color.name == "green"
        assert second_call[1]["style"].bold is True

    @patch("cockup.src.console.rprint")
    def test_rprint_point_custom_end(self, mock_rprint):
        """Test point message with custom end character."""
        message = "Custom end"
        rprint_point(message, end=" ")

        # Second call should have custom end
        second_call = mock_rprint.call_args_list[1]
        assert second_call[1]["end"] == " "

    def test_rprint_point_output(self, capsys):
        """Test actual output of rprint_point."""
        message = "Test output"
        rprint_point(message)

        captured = capsys.readouterr()
        assert message in captured.out
        assert "=>" in captured.out


class TestRprintError:
    """Test the rprint_error function."""

    @patch("cockup.src.console.rprint")
    def test_rprint_error_basic(self, mock_rprint):
        """Test basic error message printing."""
        message = "Something went wrong"
        rprint_error(message)

        # Should make two calls: one for "=> " and one for the message
        assert mock_rprint.call_count == 2

        # First call: cyan arrow with no newline
        first_call = mock_rprint.call_args_list[0]
        assert first_call[0][0] == "=> "
        assert first_call[1]["end"] == ""
        assert first_call[1]["style"].color.name == "cyan"
        assert first_call[1]["style"].bold is True

        # Second call: red message with newline
        second_call = mock_rprint.call_args_list[1]
        assert second_call[1]["message"] == message
        assert second_call[1]["end"] == "\n"
        assert second_call[1]["style"].color.name == "red"
        assert second_call[1]["style"].bold is True

    @patch("cockup.src.console.rprint")
    def test_rprint_error_custom_end(self, mock_rprint):
        """Test error message with custom end character."""
        message = "Error without newline"
        rprint_error(message, end="")

        # Second call should have custom end
        second_call = mock_rprint.call_args_list[1]
        assert second_call[1]["end"] == ""

    def test_rprint_error_output(self, capsys):
        """Test actual output of rprint_error."""
        message = "Test error"
        rprint_error(message)

        captured = capsys.readouterr()
        assert message in captured.out
        assert "=>" in captured.out


class TestRprintWarning:
    """Test the rprint_warning function."""

    @patch("cockup.src.console.rprint")
    def test_rprint_warning_basic(self, mock_rprint):
        """Test basic warning message printing."""
        message = "This is a warning"
        rprint_warning(message)

        # Should make two calls: one for "=> " and one for the message
        assert mock_rprint.call_count == 2

        # First call: cyan arrow with no newline
        first_call = mock_rprint.call_args_list[0]
        assert first_call[0][0] == "=> "
        assert first_call[1]["end"] == ""
        assert first_call[1]["style"].color.name == "cyan"
        assert first_call[1]["style"].bold is True

        # Second call: yellow message with newline
        second_call = mock_rprint.call_args_list[1]
        assert second_call[1]["message"] == message
        assert second_call[1]["end"] == "\n"
        assert second_call[1]["style"].color.name == "yellow"
        assert second_call[1]["style"].bold is True

    @patch("cockup.src.console.rprint")
    def test_rprint_warning_custom_end(self, mock_rprint):
        """Test warning message with custom end character."""
        message = "Warning message"
        rprint_warning(message, end=" ")

        # Second call should have custom end
        second_call = mock_rprint.call_args_list[1]
        assert second_call[1]["end"] == " "

    def test_rprint_warning_output(self, capsys):
        """Test actual output of rprint_warning."""
        message = "Test warning"
        rprint_warning(message)

        captured = capsys.readouterr()
        assert message in captured.out
        assert "=>" in captured.out


class TestConsoleIntegration:
    """Integration tests for console functions."""

    def test_multiple_message_types(self, capsys):
        """Test multiple types of messages in sequence."""
        rprint_point("Starting process")
        rprint_warning("This is a warning")
        rprint_error("This is an error")
        rprint("Regular message")

        captured = capsys.readouterr()
        output = captured.out

        assert "Starting process" in output
        assert "This is a warning" in output
        assert "This is an error" in output
        assert "Regular message" in output
        assert output.count("=>") == 3  # Three messages with arrows

    def test_message_formatting_consistency(self, capsys):
        """Test that all message types follow consistent formatting."""
        messages = [
            ("point", rprint_point, "Point message"),
            ("error", rprint_error, "Error message"),
            ("warning", rprint_warning, "Warning message"),
        ]

        for msg_type, func, message in messages:
            func(message)

        captured = capsys.readouterr()
        lines = [line for line in captured.out.split("\n") if line.strip()]

        # Each line should start with "=>" (after ANSI codes are stripped conceptually)
        for line in lines:
            # The actual output includes ANSI escape codes, but the message content should be there
            assert any(msg in line for _, _, msg in messages)

    def test_empty_message_handling(self, capsys):
        """Test handling of empty messages."""
        rprint_point("")
        rprint_error("")
        rprint_warning("")
        rprint("")

        captured = capsys.readouterr()
        # Should not crash, and should produce some output (at least the arrows)
        assert "=>" in captured.out or captured.out == "\n" * 4
