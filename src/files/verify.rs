use super::{paths::magic, rules::expand};
use crate::{
    config::{Config, Symlinks},
    manifest::Manifest,
    report,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn check_entry(path: &Path) -> Result<usize, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if metadata.is_dir() {
        let mut count = 1;
        for child in fs::read_dir(path).map_err(|e| format!("{}: {e}", path.display()))? {
            let child = child.map_err(|e| format!("{}: {e}", path.display()))?;
            count += check_entry(&child.path())?;
        }
        Ok(count)
    } else if metadata.is_file() {
        fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(1)
    } else if metadata.is_symlink() {
        // Reference-mode links may be dangling; never follow them during verification.
        Ok(1)
    } else {
        Err(format!("Unsupported backup entry: {}", path.display()))
    }
}

fn check_link(root: &Path, link: &crate::manifest::Link) -> Result<(), String> {
    let path = root.join(&link.backup_path);
    let mut parent = PathBuf::from(root);
    if let Some(components) = link.backup_path.parent() {
        for component in components.components() {
            parent.push(component);
            let metadata =
                fs::symlink_metadata(&parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            if !metadata.is_dir() || metadata.is_symlink() {
                return Err(format!(
                    "Backup path has a non-directory parent: {}",
                    parent.display()
                ));
            }
        }
    }
    let metadata = fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let valid = match link.mode {
        Symlinks::Referece => metadata.is_symlink(),
        Symlinks::Dereference => metadata.is_file() || metadata.is_dir(),
        Symlinks::Prompt => false,
    };
    if !valid {
        return Err(format!("Backup entry has wrong type: {}", path.display()));
    }
    if link.mode == Symlinks::Dereference {
        check_entry(&path)?;
    }
    Ok(())
}

pub(crate) fn run(cfg: &Config) -> Result<(), String> {
    if !cfg.destination.is_dir() {
        return Err(format!(
            "Backup destination is not a directory: {}",
            cfg.destination.display()
        ));
    }
    if fs::symlink_metadata(cfg.destination.join(".cockup-incomplete")).is_ok() {
        return Err("Backup is incomplete; run a successful backup before verifying".into());
    }
    let manifest = Manifest::load(&cfg.destination)?
        .ok_or_else(|| format!("Missing symlink manifest: {}", cfg.destination.display()))?;
    let mut failures = 0;
    let mut entries = 0;
    for link in &manifest.links {
        if let Err(error) = check_link(&cfg.destination, link) {
            report::error(&error);
            failures += 1;
        }
    }
    for rule in &cfg.rules {
        let mut base = rule.src.clone();
        while magic(&base) {
            base = base.parent().unwrap().to_path_buf();
        }
        let prefix = rule.src.strip_prefix(&base).unwrap();
        let backup = cfg.destination.join(&rule.to);
        for target in &rule.targets {
            let source = backup.join(prefix.join(target));
            match expand(&source) {
                Ok(paths) if paths.is_empty() => {
                    report::error(&format!("Backup path not found: {}", source.display()));
                    failures += 1;
                }
                Ok(paths) => {
                    for path in paths {
                        match check_entry(&path) {
                            Ok(count) => entries += count,
                            Err(error) => {
                                report::error(&error);
                                failures += 1;
                            }
                        }
                    }
                }
                Err(error) => {
                    report::error(&format!("{}: {error}", source.display()));
                    failures += 1;
                }
            }
        }
    }
    if failures > 0 {
        return Err(format!("Verification failed: {failures} problems found."));
    }
    report::success(&format!(
        "Verification completed: {entries} backup entries and {} symlink records checked.",
        manifest.links.len()
    ));
    Ok(())
}
