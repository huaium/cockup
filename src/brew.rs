use crate::report;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    process::Command,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
fn brew(args: &[&str]) -> Result<String, String> {
    let output = Command::new("brew")
        .args(args)
        .env("HOMEBREW_NO_AUTO_UPDATE", "1")
        .output()
        .map_err(|e| format!("Homebrew is not accessible: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "Homebrew {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}
fn cask(name: &str) -> Result<Vec<String>, String> {
    let data: Value = serde_json::from_str(&brew(&["info", "--json=v2", "--cask", name])?)
        .map_err(|e| format!("Invalid Homebrew JSON: {e}"))?;
    let record = data["casks"]
        .as_array()
        .and_then(|c| c.first())
        .ok_or("Cask not found")?;
    let artifacts = record["artifacts"]
        .as_array()
        .ok_or("Cask artifacts missing")?;
    let mut paths = Vec::new();
    for artifact in artifacts {
        if let Some(zap) = artifact.get("zap").and_then(Value::as_array) {
            for item in zap {
                for key in ["rmdir", "trash"] {
                    match item.get(key) {
                        Some(Value::String(path)) => paths.push(path.clone()),
                        Some(Value::Array(items)) => {
                            for path in items {
                                if let Some(path) = path.as_str() {
                                    paths.push(path.to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(paths)
}
pub fn list(mut names: Vec<String>) -> Result<(), String> {
    brew(&["--version"])?;
    if names.is_empty() {
        names = brew(&["list", "--casks"])?
            .lines()
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
    }
    let count = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(32)
        .min(names.len());
    let next = AtomicUsize::new(0);
    let results = Mutex::new(BTreeMap::new());
    std::thread::scope(|scope| {
        for _ in 0..count {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(name) = names.get(i) else {
                        break;
                    };
                    let result = cask(name);
                    results.lock().unwrap().insert(name.clone(), result);
                }
            });
        }
    });
    let mut failed = 0;
    let mut found = false;
    for (name, result) in results.into_inner().unwrap() {
        match result {
            Ok(paths) if !paths.is_empty() => {
                found = true;
                println!("{name}:");
                for path in paths {
                    println!("  {path}");
                }
            }
            Err(e) => {
                report::error(&format!("Cask `{name}`: {e}"));
                failed += 1;
            }
            _ => {}
        }
    }
    if failed > 0 {
        return Err(format!(
            "Homebrew discovery completed with {failed} failures."
        ));
    }
    if !found {
        println!("No potential configs found.");
    }
    Ok(())
}
