use super::{
    copy::copy,
    paths::{check_overlap, remove, resolved},
    state::CopyState,
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

pub(super) fn restore_link(
    src: &Path,
    dst: &Path,
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
        let updating = fs::symlink_metadata(dst).is_ok();
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
                    Symlinks::Referece,
                    ancestors,
                    state,
                )?;
                if state.dry_run {
                    for entry in link.target_chain.iter().rev() {
                        let location = state.mapped(&entry.location);
                        println!(
                            "Would restore symlink: {} -> {}",
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
                        fs::create_dir_all(location.parent().unwrap())
                            .map_err(|e| e.to_string())?;
                        remove(&location).map_err(|e| e.to_string())?;
                        std::os::unix::fs::symlink(
                            state.link_target(&entry.location, &entry.target, &location),
                            &location,
                        )
                        .map_err(|e| e.to_string())?;
                    }
                }
            }
            if state.dry_run {
                println!(
                    "Would {} symlink: {} -> {}",
                    if updating { "replace" } else { "restore" },
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
            remove(dst).map_err(|e| e.to_string())?;
            std::os::unix::fs::symlink(state.link_target(&link.location, &link.target, dst), dst)
                .map_err(|e| e.to_string())?;
            Ok(())
        })();
        state
            .manifest
            .links
            .insert(index.min(state.manifest.links.len()), link);
        if result.is_ok() && !state.dry_run {
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
            let prompt = format!(
                "Symlink {} -> {}:\n[r]eferece link or [d]ereference target? (applies to all symlinks)",
                src.display(),
                fs::read_link(src).map_err(|e| e.to_string())?.display()
            );
            loop {
                report::warning(&prompt);
                match report::input("")?.to_lowercase().as_str() {
                    "r" | "referece" => {
                        state.prompt_choice = Some(Ok(Symlinks::Referece));
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
