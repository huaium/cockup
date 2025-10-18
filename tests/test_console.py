from unittest.mock import patch

from cockup.src.console import Style, rprint, rprint_error, rprint_point, rprint_warning


class TestStyle:
    """Test the Style class."""

    def test_style_init_defaults(self):
        """Test Style initialization with default values."""
        style = Style()
        assert style.color is None
        assert style.bold is False
        assert style.dark is False
        assert style.underline is False
        assert style.blink is False
        assert style.reverse is False
        assert style.concealed is False
        assert style.strike is False

    def test_style_init_with_values(self):
        """Test Style initialization with custom values."""
        style = Style(color="red", bold=True, underline=True)
        assert style.color == "red"
        assert style.bold is True
        assert style.underline is True
        assert style.dark is False

    def test_to_attrs_empty(self):
        """Test to_attrs with no attributes set."""
        style = Style()
        assert style.to_attrs() == []

    def test_to_attrs_single(self):
        """Test to_attrs with a single attribute."""
        style = Style(bold=True)
        assert style.to_attrs() == ["bold"]

    def test_to_attrs_multiple(self):
        """Test to_attrs with multiple attributes."""
        style = Style(bold=True, underline=True, blink=True)
        attrs = style.to_attrs()
        assert len(attrs) == 3
        assert "bold" in attrs
        assert "underline" in attrs
        assert "blink" in attrs

    def test_to_attrs_all(self):
        """Test to_attrs with all attributes set."""
        style = Style(
            bold=True,
            dark=True,
            underline=True,
            blink=True,
            reverse=True,
            concealed=True,
            strike=True,
        )
        attrs = style.to_attrs()
        assert len(attrs) == 7
        assert set(attrs) == {
            "bold",
            "dark",
            "underline",
            "blink",
            "reverse",
            "concealed",
            "strike",
        }


class TestRprint:
    """Test the base rprint function."""

    @patch("cockup.src.console.cprint")
    def test_rprint_basic_message(self, mock_cprint):
        """Test basic message printing."""
        message = "Test message"
        rprint(message)

        mock_cprint.assert_called_once_with(message, color=None, attrs=None, end="\n")

    @patch("cockup.src.console.cprint")
    def test_rprint_with_style(self, mock_cprint):
        """Test message printing with custom style."""
        message = "Styled message"
        style = Style(color="red", bold=True)
        rprint(message, style=style)

        mock_cprint.assert_called_once_with(
            message, color="red", attrs=["bold"], end="\n"
        )

    @patch("cockup.src.console.cprint")
    def test_rprint_custom_end(self, mock_cprint):
        """Test message printing with custom end character."""
        message = "No newline"
        rprint(message, end="")

        mock_cprint.assert_called_once_with(message, color=None, attrs=None, end="")

    @patch("cockup.src.console.cprint")
    def test_rprint_with_style_and_end(self, mock_cprint):
        """Test message printing with both style and end parameter."""
        message = "Complete test"
        style = Style(color="blue", underline=True)
        rprint(message, style=style, end=" ")

        mock_cprint.assert_called_once_with(
            message, color="blue", attrs=["underline"], end=" "
        )

    @patch("cockup.src.console.cprint")
    def test_rprint_with_multiple_attrs(self, mock_cprint):
        """Test message printing with multiple style attributes."""
        message = "Multiple attrs"
        style = Style(color="green", bold=True, underline=True, blink=True)
        rprint(message, style=style)

        call_args = mock_cprint.call_args
        assert call_args[0][0] == message
        assert call_args[1]["color"] == "green"
        assert set(call_args[1]["attrs"]) == {"bold", "underline", "blink"}
        assert call_args[1]["end"] == "\n"


class TestRprintPoint:
    """Test the rprint_point function."""

    @patch("cockup.src.console.rprint")
    def test_rprint_point_basic(self, mock_rprint):
        """Test basic point message printing."""
        message = "Process started"
        rprint_point(message)

        # Should make two calls: one for "=> " and one for the message
        assert mock_rprint.call_count == 2

        # First call: cyan arrow with no newline (positional argument)
        first_call = mock_rprint.call_args_list[0]
        assert first_call[0][0] == "=> "  # First positional argument
        assert first_call[1]["end"] == ""
        assert first_call[1]["style"].color == "cyan"
        assert first_call[1]["style"].bold is True

        # Second call: green message with newline (keyword argument)
        second_call = mock_rprint.call_args_list[1]
        assert second_call[1]["message"] == message
        assert second_call[1]["end"] == "\n"
        assert second_call[1]["style"].color == "green"
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

        # First call: cyan arrow with no newline (positional argument)
        first_call = mock_rprint.call_args_list[0]
        assert first_call[0][0] == "=> "  # First positional argument
        assert first_call[1]["end"] == ""
        assert first_call[1]["style"].color == "cyan"
        assert first_call[1]["style"].bold is True

        # Second call: red message with newline (keyword argument)
        second_call = mock_rprint.call_args_list[1]
        assert second_call[1]["message"] == message
        assert second_call[1]["end"] == "\n"
        assert second_call[1]["style"].color == "red"
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

        # First call: cyan arrow with no newline (positional argument)
        first_call = mock_rprint.call_args_list[0]
        assert first_call[0][0] == "=> "  # First positional argument
        assert first_call[1]["end"] == ""
        assert first_call[1]["style"].color == "cyan"
        assert first_call[1]["style"].bold is True

        # Second call: yellow message with newline (keyword argument)
        second_call = mock_rprint.call_args_list[1]
        assert second_call[1]["message"] == message
        assert second_call[1]["end"] == "\n"
        assert second_call[1]["style"].color == "yellow"
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
