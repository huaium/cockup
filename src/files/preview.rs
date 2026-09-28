use super::{rules::rule, state::CopyState};
use crate::{
    config::{Config, Symlinks},
    manifest, report,
};
use std::fs;

pub(super) fn run(cfg: &Config, state: &mut CopyState) -> Result<(), String> {
    if state.restore && !cfg.destination.is_dir() {
        return Err("Backup destination is not a directory".into());
    }
    if !state.restore && fs::symlink_metadata(&cfg.destination).is_ok() && !cfg.destination.is_dir()
    {
        return Err("Backup destination is not a directory".into());
    }
    println!(
        "Dry run: {} (no files changed; hooks skipped)",
        if state.restore { "restore" } else { "backup" }
    );
    if !state.restore && cfg.clean && cfg.destination.is_dir() {
        for entry in fs::read_dir(&cfg.destination).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_name() != manifest::NAME && entry.file_name() != ".cockup-incomplete" {
                println!("Would remove: {}", entry.path().display());
            }
        }
    }
    state.planning = true;
    let mut failures = 0;
    for config_rule in &cfg.rules {
        failures += rule(
            config_rule,
            &cfg.destination,
            state.restore,
            config_rule.metadata.unwrap(),
            if state.restore {
                Symlinks::Referece
            } else {
                config_rule.symlinks.unwrap()
            },
            state,
        );
    }
    if failures > 0 {
        return Err(format!("Dry run completed with {failures} failures."));
    }
    report::success("Dry run completed; no files changed.");
    Ok(())
}
