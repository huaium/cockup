use crate::{config::Hook, report};
use std::{
    path::Path,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
};
unsafe extern "C" {
    fn signal(signal: i32, handler: usize) -> usize;
}
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
extern "C" fn interrupt(_signal: i32) {
    INTERRUPTED.store(true, Ordering::Relaxed);
}
struct InterruptGuard(usize);
impl InterruptGuard {
    fn install() -> Result<Self, String> {
        // SAFETY: SIGINT is 2 on our macOS targets. The handler only sets a
        // lock-free atomic flag, and is restored when this sequential stage ends.
        let previous = unsafe { signal(2, interrupt as *const () as usize) };
        if previous == usize::MAX {
            Err(std::io::Error::last_os_error().to_string())
        } else {
            Ok(Self(previous))
        }
    }
}
impl Drop for InterruptGuard {
    fn drop(&mut self) {
        // SAFETY: restore the handler returned by signal() above.
        unsafe {
            signal(2, self.0);
        }
    }
}
pub(crate) fn check_interrupted() -> Result<(), String> {
    if INTERRUPTED.load(Ordering::Relaxed) {
        Err("Hook execution interrupted.".into())
    } else {
        Ok(())
    }
}
fn stop(child: &mut Child) -> Result<(), String> {
    let killed = child.kill();
    let reaped = child.wait();
    killed.map_err(|error| error.to_string())?;
    reaped.map_err(|error| error.to_string())?;
    Ok(())
}
fn command(script: &str, hook: &Hook, directory: &Path) -> Result<(), String> {
    check_interrupted()?;
    let mut cmd = Command::new("/bin/sh");
    cmd.args(["-cu", script])
        .envs(&hook.env)
        .current_dir(directory);
    if !hook.output {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                let _ = stop(&mut child);
                return Err(error.to_string());
            }
            Ok(None) => {}
        }
        if hook
            .timeout
            .is_some_and(|seconds| start.elapsed() >= std::time::Duration::from_secs(seconds))
        {
            stop(&mut child)?;
            return Err(format!("timed out after {} seconds", hook.timeout.unwrap()));
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    check_interrupted()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("exited with {status}"))
    }
}
pub(crate) fn directory<'a>(hook: &'a Hook, fallback: &'a Path) -> Result<&'a Path, String> {
    let directory = hook.cwd.as_deref().unwrap_or(fallback);
    if hook.cwd.is_some() && !directory.is_dir() {
        return Err(format!(
            "cwd is not an accessible directory: {}",
            directory.display()
        ));
    }
    Ok(directory)
}
pub fn run(hooks: &[Hook], directory: &Path) -> usize {
    if check_interrupted().is_err() {
        return 1;
    }
    let _guard = match InterruptGuard::install() {
        Ok(guard) => guard,
        Err(error) => {
            report::error(&format!("Cannot handle hook interruption: {error}"));
            return 1;
        }
    };
    let mut failed = 0;
    for (i, hook) in hooks.iter().enumerate() {
        if check_interrupted().is_err() {
            return failed.max(1);
        }
        report::success(&format!(
            "Running hook ({}/{}): {}",
            i + 1,
            hooks.len(),
            hook.name
        ));
        let result = self::directory(hook, directory).and_then(|directory| {
            command(&hook.command, hook, directory)
                .map_err(|e| format!("command: {e}"))
                .and_then(|()| match &hook.check {
                    Some(check) => {
                        command(check, hook, directory).map_err(|e| format!("check: {e}"))
                    }
                    None => Ok(()),
                })
        });
        if let Err(e) = result {
            report::error(&format!("Hook `{}` {e}", hook.name));
            failed += 1;
        }
        if check_interrupted().is_err() {
            report::error("Hook execution interrupted; remaining hooks skipped.");
            return failed.max(1);
        }
    }
    let noun = if hooks.len() == 1 { "hook" } else { "hooks" };
    let summary = format!("Completed {}/{} {noun}.", hooks.len() - failed, hooks.len());
    if failed > 0 {
        report::error(&format!(
            "{summary} Error: {failed} {} failed.",
            if failed == 1 { "hook" } else { "hooks" }
        ));
    } else if !hooks.is_empty() {
        report::success(&summary);
    }
    failed
}

pub fn select(cfg: &crate::config::Config, name: Option<&str>) -> Result<(), String> {
    // Preserve first-seen display order, with the last definition winning duplicate names.
    let mut available: Vec<Hook> = Vec::new();
    let rule_hooks = cfg
        .rules
        .iter()
        .flat_map(|r| r.on_start.iter().chain(&r.on_end));
    let defaults = rule_hooks
        .map(|h| (h, &cfg.destination))
        .chain(cfg.hooks.pre_backup.iter().map(|h| (h, &cfg.directory)))
        .chain(cfg.hooks.post_backup.iter().map(|h| (h, &cfg.destination)))
        .chain(cfg.hooks.pre_restore.iter().map(|h| (h, &cfg.directory)))
        .chain(cfg.hooks.post_restore.iter().map(|h| (h, &cfg.destination)));
    for (hook, directory) in defaults {
        let mut hook = hook.clone();
        hook.cwd.get_or_insert_with(|| directory.clone());
        if let Some(existing) = available.iter_mut().find(|h| h.name == hook.name) {
            *existing = hook;
        } else {
            available.push(hook);
        }
    }
    let selected = if let Some(name) = name {
        vec![
            available
                .iter()
                .find(|h| h.name == name)
                .ok_or_else(|| format!("Hook `{name}` not found"))?
                .clone(),
        ]
    } else {
        if available.is_empty() {
            return Err("No hooks defined in the configuration.".into());
        }
        println!("Available hooks:");
        for (i, h) in available.iter().enumerate() {
            println!("[{}] {}", i + 1, h.name);
        }
        loop {
            let choices = report::input("Select hooks (separate by comma): ")?;
            let selected: Option<Vec<Hook>> = choices
                .split(',')
                .map(str::trim)
                .map(|choice| {
                    choice
                        .parse::<usize>()
                        .ok()
                        .and_then(|index| available.get(index.wrapping_sub(1)))
                        .cloned()
                })
                .collect();
            if let Some(selected) = selected {
                break selected;
            }
            report::warning("Invalid hook selection; enter hook numbers separated by commas.");
        }
    };
    // Resolve missing directories before executing any selected commands.
    for hook in &selected {
        let directory = hook.cwd.as_deref().unwrap();
        if !directory.is_dir() {
            match std::fs::symlink_metadata(directory) {
                Ok(_) => {
                    return Err(format!(
                        "Hook `{}` cwd is not a directory: {}",
                        hook.name,
                        directory.display()
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    report::warning(&format!(
                        "Hook `{}` working directory does not exist: {}",
                        hook.name,
                        directory.display()
                    ));
                    if !report::confirm_with_prompt("Create working directory? [y/N]: ")? {
                        report::warning("Hook execution cancelled; no hooks were run.");
                        return Ok(());
                    }
                    std::fs::create_dir_all(directory).map_err(|error| {
                        format!(
                            "Cannot create hook working directory {}: {error}",
                            directory.display()
                        )
                    })?;
                }
                Err(error) => {
                    return Err(format!(
                        "Cannot access hook working directory {}: {error}",
                        directory.display()
                    ));
                }
            }
        }
    }
    if run(&selected, &cfg.directory) > 0 {
        Err("Hook execution failed.".into())
    } else {
        Ok(())
    }
}
