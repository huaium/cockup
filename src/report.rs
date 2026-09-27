use std::io::{self, IsTerminal, Write};
fn colored(message: &str, color: u8, error: bool) {
    let terminal = if error {
        io::stderr().is_terminal()
    } else {
        io::stdout().is_terminal()
    };
    let use_color = std::env::var_os("NO_COLOR").is_none()
        && (terminal || std::env::var("FORCE_COLOR").is_ok_and(|v| v != "0"));
    let line = if use_color {
        format!("\x1b[{color};1m=> {message}\x1b[0m")
    } else {
        format!("=> {message}")
    };
    if error {
        eprintln!("{line}");
    } else {
        println!("{line}");
    }
}
pub fn success(message: &str) {
    colored(message, 32, false);
}
pub fn error(message: &str) {
    colored(message, 31, true);
}
pub fn warning(message: &str) {
    colored(message, 33, true);
}
pub fn input(prompt: &str) -> Result<String, String> {
    print!("{prompt}");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| e.to_string())?;
    Ok(input.trim().to_string())
}
pub fn confirm() -> Result<bool, String> {
    Ok(matches!(
        input("Continue? [y/N]: ")?.to_lowercase().as_str(),
        "y" | "yes"
    ))
}
