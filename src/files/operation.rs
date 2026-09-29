use super::{
    paths::{clean_backup, magic, resolved},
    preview,
    rules::rule,
    state::CopyState,
};
use crate::{
    config::{Config, Symlinks},
    hooks,
    manifest::Manifest,
    report,
};
use std::{fs, path::Path};

pub(crate) fn execute(cfg: &Config, restore: bool, dry_run: bool) -> Result<(), String> {
    let destination_existed = fs::symlink_metadata(&cfg.destination).is_ok();
    let incomplete = cfg.destination.join(".cockup-incomplete");
    if restore && fs::symlink_metadata(&incomplete).is_ok() {
        return Err("Backup is incomplete; run a successful backup before restoring".into());
    }
    let stored = Manifest::load(&cfg.destination)?;
    if restore
        && stored.is_none()
        && cfg
            .rules
            .iter()
            .any(|r| r.symlinks != Some(Symlinks::Reference))
    {
        return Err(
            "Missing symlink manifest: cannot restore dereference/prompt backup safely".into(),
        );
    }
    let mut state = CopyState {
        root: cfg.destination.clone(),
        manifest: if !restore && cfg.clean {
            Manifest::new()?
        } else {
            stored.unwrap_or(Manifest::new()?)
        },
        restore,
        planning: false,
        dry_run,
        clean: cfg.clean,
        planned: Vec::new(),
        replacing_directories: Vec::new(),
        prompt_choice: None,
        home_mapping: None,
    };
    if restore {
        state.choose_home(cfg, "Restore")?;
    }
    let destination = resolved(&cfg.destination).map_err(|e| e.to_string())?;
    if !restore && cfg.clean {
        if destination.parent().is_none()
            || std::env::var_os("HOME")
                .is_some_and(|h| resolved(Path::new(&h)).is_ok_and(|h| h == destination))
        {
            return Err("Refusing to clean the filesystem root or home directory".into());
        }
        for r in &cfg.rules {
            let mut base = r.src.clone();
            while magic(&base) {
                base = base.parent().unwrap().to_path_buf();
            }
            if resolved(&base)
                .map_err(|e| e.to_string())?
                .starts_with(&destination)
            {
                return Err("Clean destination overlaps a source directory".into());
            }
        }
    }
    if dry_run {
        return preview::run(cfg, &mut state);
    }
    if !restore {
        state.planning = true;
        let mut planning_failures = 0;
        for rule_config in &cfg.rules {
            planning_failures += rule(
                rule_config,
                &cfg.destination,
                false,
                rule_config.metadata.unwrap(),
                rule_config.symlinks.unwrap(),
                &mut state,
            );
        }
        state.planning = false;
        if planning_failures > 0
            && (cfg.clean
                || cfg
                    .rules
                    .iter()
                    .any(|r| r.symlinks == Some(Symlinks::Prompt)))
        {
            return Err("Symlink planning failed; backup destination was not modified".into());
        }
    }
    report::success(if restore {
        "Starting restore..."
    } else {
        "Starting backup..."
    });
    let pre_hooks = if restore {
        &cfg.hooks.pre_restore
    } else {
        &cfg.hooks.pre_backup
    };
    if !pre_hooks.is_empty() {
        report::success(if restore {
            "Running pre-restore hooks..."
        } else {
            "Running pre-backup hooks..."
        });
    }
    let mut failures = hooks::run(pre_hooks, &cfg.directory);
    if failures > 0 {
        return Err(format!(
            "{} stopped: {failures} pre-{} {} failed.",
            if restore { "Restore" } else { "Backup" },
            if restore { "restore" } else { "backup" },
            if failures == 1 { "hook" } else { "hooks" }
        ));
    }
    if !restore {
        fs::create_dir_all(&cfg.destination).map_err(|e| e.to_string())?;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&incomplete)
        {
            Ok(file) => file.sync_all().map_err(|e| e.to_string())?,
            Err(e)
                if e.kind() == std::io::ErrorKind::AlreadyExists
                    && fs::symlink_metadata(&incomplete).is_ok_and(|m| m.is_file()) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    if restore {
        if !cfg.destination.is_dir() {
            return Err("Backup destination is not a directory".into());
        }
    } else {
        if cfg.clean {
            report::success("Clean mode enabled, will remove backup folder first if exists.");
            report::success(if destination_existed {
                "Found existing backup folder, removing..."
            } else {
                "Existing backup folder not found, creating a new one."
            });
            clean_backup(&cfg.destination).map_err(|e| e.to_string())?;
        } else {
            report::success(
                "Clean mode disabled, will not remove existing backup folder, just update.",
            );
        }
        fs::create_dir_all(&cfg.destination).map_err(|e| e.to_string())?;
    }
    report::success(
        if cfg.rules.iter().any(|r| r.metadata != Some(cfg.metadata)) {
            "Metadata preservation varies by rule."
        } else if cfg.metadata {
            "Metadata preservation enabled."
        } else {
            "Metadata preservation disabled."
        },
    );
    for (index, r) in cfg.rules.iter().enumerate() {
        if !r.on_start.is_empty() {
            report::success(&format!("Running pre-rule hooks for Rule {}...", index + 1));
        }
        failures += hooks::run(&r.on_start, &cfg.destination);
        failures += rule(
            r,
            &cfg.destination,
            restore,
            r.metadata.unwrap(),
            if restore {
                Symlinks::Reference
            } else {
                r.symlinks.unwrap()
            },
            &mut state,
        );
        if !r.on_end.is_empty() {
            report::success(&format!(
                "Running post-rule hooks for Rule {}...",
                index + 1
            ));
        }
        failures += hooks::run(&r.on_end, &cfg.destination);
    }
    let post_hooks = if restore {
        &cfg.hooks.post_restore
    } else {
        &cfg.hooks.post_backup
    };
    if !post_hooks.is_empty() {
        report::success(if restore {
            "Running post-restore hooks..."
        } else {
            "Running post-backup hooks..."
        });
    }
    failures += hooks::run(post_hooks, &cfg.destination);
    if failures > 0 {
        return Err(format!("Operation completed with {failures} failures."));
    }
    if !restore {
        state.manifest.ingredients = cfg.ingredients.clone();
        state.manifest.save(&cfg.destination)?;
        fs::remove_file(&incomplete).map_err(|e| e.to_string())?;
    }
    report::success(if restore {
        "Restore completed."
    } else {
        "Backup completed."
    });
    Ok(())
}
