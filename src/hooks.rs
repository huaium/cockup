use crate::{config::Hook, report};
use std::{
    path::Path,
    process::{Command, Stdio},
};
fn command(args: &[String], hook: &Hook, directory: &Path) -> Result<(), String> {
    let mut cmd = Command::new(&args[0]);
    cmd.args(&args[1..]).envs(&hook.env).current_dir(directory);
    if !hook.output {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.to_string());
            }
            Ok(None) => {}
        }
        if hook
            .timeout
            .is_some_and(|seconds| start.elapsed() >= std::time::Duration::from_secs(seconds))
        {
            let _ = child.kill();
            child.wait().map_err(|e| e.to_string())?;
            return Err(format!("timed out after {} seconds", hook.timeout.unwrap()));
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    if status.success() {
        Ok(())
    } else {
        Err(format!("exited with {status}"))
    }
}
pub fn run(hooks: &[Hook], directory: &Path) -> usize {
    let mut failed = 0;
    for (i, hook) in hooks.iter().enumerate() {
        report::success(&format!(
            "Running hook ({}/{}): {}",
            i + 1,
            hooks.len(),
            hook.name
        ));
        let result = command(&hook.command, hook, directory)
            .map_err(|e| format!("command: {e}"))
            .and_then(|()| match &hook.check {
                Some(check) => command(check, hook, directory).map_err(|e| format!("check: {e}")),
                None => Ok(()),
            });
        if let Err(e) = result {
            report::error(&format!("Hook `{}` {e}", hook.name));
            failed += 1;
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
    for hook in cfg.all_hooks() {
        if let Some(existing) = available.iter_mut().find(|h| h.name == hook.name) {
            *existing = hook.clone();
        } else {
            available.push(hook.clone());
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
    if run(&selected, &cfg.directory) > 0 {
        Err("Hook execution failed.".into())
    } else {
        Ok(())
    }
}
