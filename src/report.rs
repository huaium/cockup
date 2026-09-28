use owo_colors::{OwoColorize, Stream, Style};
use std::io::{self, Write};

fn colors_enabled() -> bool {
    // Preserve Cockup's existing precedence when both overrides are present.
    std::env::var_os("NO_COLOR").is_none()
}

pub fn app_name(name: &str) {
    let heading = format!("{name}:");
    if colors_enabled() {
        println!(
            "{}",
            heading.if_supports_color(Stream::Stdout, |text| text
                .style(Style::new().green().bold()))
        );
    } else {
        println!("{heading}");
    }
}
fn point(message: &str, style: Style, error: bool) {
    let stream = if error {
        Stream::Stderr
    } else {
        Stream::Stdout
    };
    if colors_enabled() {
        let prefix = "=> ";
        if error {
            eprintln!(
                "{}{}",
                prefix.if_supports_color(stream, |text| text.style(Style::new().cyan().bold())),
                message.if_supports_color(stream, |text| text.style(style))
            );
        } else {
            println!(
                "{}{}",
                prefix.if_supports_color(stream, |text| text.style(Style::new().cyan().bold())),
                message.if_supports_color(stream, |text| text.style(style))
            );
        }
    } else if error {
        eprintln!("=> {message}");
    } else {
        println!("=> {message}");
    }
}
pub fn success(message: &str) {
    point(message, Style::new().green().bold(), false);
}
pub fn error(message: &str) {
    if let Some((first, second)) = message.split_once("\n=> ") {
        point(first, Style::new().red().bold(), true);
        point(second, Style::new().red().bold(), true);
    } else {
        point(message, Style::new().red().bold(), true);
    }
}
pub fn warning(message: &str) {
    point(message, Style::new().yellow().bold(), true);
}
pub fn display_path(path: &std::path::Path) -> String {
    if let Some(home) = std::env::var_os("HOME")
        && let Ok(suffix) = path.strip_prefix(home)
    {
        return if suffix.as_os_str().is_empty() {
            "~".into()
        } else {
            format!("~/{}", suffix.display())
        };
    }
    path.display().to_string()
}
pub fn copied(kind: &str, updating: bool, path: &std::path::Path) {
    let label = if updating {
        format!("{kind} existed, updating:")
    } else {
        format!("{kind} copied:")
    };
    if colors_enabled() {
        println!(
            "{} {}",
            label.if_supports_color(Stream::Stdout, |text| text.style(Style::new().bold())),
            display_path(path)
        );
    } else {
        println!("{} {}", label, display_path(path));
    }
}
pub fn input(prompt: &str) -> Result<String, String> {
    print!("{prompt}");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut input = String::new();
    let read = io::stdin()
        .read_line(&mut input)
        .map_err(|e| e.to_string())?;
    if read == 0 {
        return Err("Input ended before a valid choice was made".into());
    }
    Ok(input.trim().to_string())
}
pub fn input_bold(prompt: &str) -> Result<String, String> {
    if colors_enabled() {
        input(&format!(
            "{}",
            prompt.if_supports_color(Stream::Stdout, |text| text
                .style(Style::new().white().bold()))
        ))
    } else {
        input(prompt)
    }
}
pub fn confirm() -> Result<bool, String> {
    loop {
        match input_bold("Continue? [y/N]: ")?.to_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "" | "n" | "no" => return Ok(false),
            _ => warning("Please enter y or n."),
        }
    }
}
