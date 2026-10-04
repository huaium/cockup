use super::{
    copy::copy,
    paths::{check_overlap, remove, resolved},
    state::{CopyState, RestorePolicy},
};
use crate::{
    config::Symlinks,
    manifest::{Link, TargetLink},
    report,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn review_link(dst: &Path, target: &Path, state: &CopyState) -> Result<bool, String> {
    if state.dry_run {
        return Ok(true);
    }
    let existing = match fs::symlink_metadata(dst) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(error) => return Err(format!("{}: {error}", dst.display())),
    };
    match state.restore_policy {
        RestorePolicy::Override => return Ok(true),
        RestorePolicy::SkipExisting => {
            report::warning(&format!(
                "Keeping existing local path: {}",
                report::display_path(dst)
            ));
            return Ok(false);
        }
        RestorePolicy::Ask => {}
    }
    let current = if existing.is_symlink() {
        let link = fs::read_link(dst).map_err(|e| e.to_string())?;
        if link == target {
            return Ok(true);
        }
        format!("link -> {}", link.display())
    } else {
        "not a symlink".into()
    };
    report::warning(&format!(
        "Existing local path differs: {}",
        report::display_path(dst)
    ));
    report::diff_lines(&format!("-link -> {}\n+{current}\n", target.display()));
    let overwrite = report::confirm_with_prompt("Overwrite local file? [y/N]: ")?;
    if !overwrite {
        report::warning(&format!(
            "Keeping local path: {}",
            report::display_path(dst)
        ));
    }
    Ok(overwrite)
}

pub(super) fn restore_link(
    src: &Path,
    _dst: &Path,
    metadata: bool,
    ancestors: &[PathBuf],
    state: &mut CopyState,
    key: &Path,
) -> Option<Result<(), String>> {
    if let Some(index) = state
        .manifest
        .links
        .iter()
        .position(|l| l.backup_path == key)
    {
        let link = state.manifest.links.remove(index);
        let location = state.mapped(&link.location);
        let dst = location.as_path();
        let updating = fs::symlink_metadata(dst).is_ok();
        let mut restored = false;
        let result = (|| -> Result<(), String> {
            check_overlap(src, dst)?;
            if link.mode == Symlinks::Dereference {
                for entry in &link.target_chain {
                    check_overlap(src, &state.mapped(&entry.location))?;
                }
                copy(
                    src,
                    &state.mapped(&link.resolved_target),
                    metadata,
                    Symlinks::Reference,
                    ancestors,
                    state,
                )?;
                if state.dry_run {
                    for entry in link.target_chain.iter().rev() {
                        let location = state.mapped(&entry.location);
                        let action = if state.restore_policy == RestorePolicy::SkipExisting
                            && fs::symlink_metadata(&location).is_ok()
                        {
                            "keep"
                        } else {
                            "restore"
                        };
                        println!(
                            "Would {action} symlink: {} -> {}",
                            location.display(),
                            state
                                .link_target(&entry.location, &entry.target, &location)
                                .display()
                        );
                    }
                }
                if !state.dry_run {
                    for entry in link.target_chain.iter().rev() {
                        let location = state.mapped(&entry.location);
                        let target = state.link_target(&entry.location, &entry.target, &location);
                        if !review_link(&location, &target, state)? {
                            continue;
                        }
                        fs::create_dir_all(location.parent().unwrap())
                            .map_err(|e| e.to_string())?;
                        remove(&location).map_err(|e| e.to_string())?;
                        std::os::unix::fs::symlink(&target, &location)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            if state.dry_run {
                let action = if updating && state.restore_policy == RestorePolicy::SkipExisting {
                    "keep"
                } else if updating {
                    "replace"
                } else {
                    "restore"
                };
                println!(
                    "Would {action} symlink: {} -> {}",
                    dst.display(),
                    state
                        .link_target(&link.location, &link.target, dst)
                        .display()
                );
                return Ok(());
            }
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let target = state.link_target(&link.location, &link.target, dst);
            if !review_link(dst, &target, state)? {
                return Ok(());
            }
            remove(dst).map_err(|e| e.to_string())?;
            std::os::unix::fs::symlink(&target, dst).map_err(|e| e.to_string())?;
            restored = true;
            Ok(())
        })();
        state
            .manifest
            .links
            .insert(index.min(state.manifest.links.len()), link);
        if result.is_ok() && restored {
            report::copied("Symlink", updating, dst);
        }
        return Some(result);
    }
    None
}

pub(super) fn choose_mode(
    src: &Path,
    key: &Path,
    symlinks: Symlinks,
    link: bool,
    state: &mut CopyState,
) -> Result<Symlinks, String> {
    let planned = state.planned.iter().find(|l| l.backup_path == key);
    let choice = if link
        && !state.planning
        && !state.restore
        && let Some(planned) = planned
    {
        planned.mode
    } else if link && symlinks == Symlinks::Prompt {
        if state.prompt_choice.is_none() {
            let warning = format!(
                "Symlink {} -> {}:",
                src.display(),
                fs::read_link(src).map_err(|e| e.to_string())?.display()
            );
            loop {
                report::warning(&warning);
                match report::input_bold(
                    "[r]eference link or [d]ereference target? (applies to all symlinks): ",
                )?
                .to_lowercase()
                .as_str()
                {
                    "r" | "reference" => {
                        state.prompt_choice = Some(Ok(Symlinks::Reference));
                        break;
                    }
                    "d" | "dereference" => {
                        state.prompt_choice = Some(Ok(Symlinks::Dereference));
                        break;
                    }
                    _ => report::warning("Please enter r or d."),
                }
            }
        }
        state.prompt_choice.as_ref().unwrap().clone()?
    } else {
        symlinks
    };
    Ok(choice)
}

pub(super) fn record_link(
    src: &Path,
    key: &Path,
    choice: Symlinks,
    link: bool,
    state: &CopyState,
) -> Result<Option<Link>, String> {
    let record = if link && !state.restore {
        let target = fs::read_link(src).map_err(|e| e.to_string())?;
        let target_path = if target.is_absolute() {
            target.clone()
        } else {
            src.parent().unwrap().join(&target)
        };
        let mut target_chain = Vec::new();
        if choice == Symlinks::Dereference {
            let mut next = target_path.clone();
            while fs::symlink_metadata(&next)
                .map_err(|e| e.to_string())?
                .is_symlink()
            {
                let target = fs::read_link(&next).map_err(|e| e.to_string())?;
                target_chain.push(TargetLink {
                    location: next.clone(),
                    target: target.clone(),
                });
                next = if target.is_absolute() {
                    target
                } else {
                    next.parent().unwrap().join(target)
                };
            }
        }
        Some(Link {
            location: src.to_path_buf(),
            target,
            resolved_target: resolved(&target_path).map_err(|e| e.to_string())?,
            mode: choice,
            backup_path: key.to_path_buf(),
            target_chain,
        })
    } else {
        None
    };
    Ok(record)
}
