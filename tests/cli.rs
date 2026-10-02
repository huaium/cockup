use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

fn cli(dir: &Path, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cockup"))
        .current_dir(dir)
        .args(args)
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}
fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}
fn write(dir: &Path, path: &str, content: &str) {
    let path = dir.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}
fn executable(dir: &Path, name: &str, script: &str) {
    use std::os::unix::fs::PermissionsExt;
    write(dir, name, script);
    fs::set_permissions(dir.join(name), fs::Permissions::from_mode(0o755)).unwrap();
}

#[path = "cli/brew.rs"]
mod brew;
#[path = "cli/cache.rs"]
mod cache;
#[path = "cli/config.rs"]
mod config;
#[path = "cli/diff.rs"]
mod diff;
#[path = "cli/files.rs"]
mod files;
#[path = "cli/hooks.rs"]
mod hooks;
#[path = "cli/ingredient_search.rs"]
mod ingredient_search;
#[path = "cli/ingredients.rs"]
mod ingredients;
#[path = "cli/output.rs"]
mod output;
#[path = "cli/remote_ingredients.rs"]
mod remote_ingredients;
#[path = "cli/symlink_backup.rs"]
mod symlink_backup;
#[path = "cli/symlink_restore.rs"]
mod symlink_restore;
#[path = "cli/verify.rs"]
mod verify;
