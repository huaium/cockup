use super::{
    paths::magic,
    rules::expand,
    verify::{check_entry, check_link},
};
use crate::{
    config::{Config, Symlinks},
    manifest::{Link, Manifest},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn consistent(cfg: &Config, manifest: &Manifest, link: &Link) -> bool {
    cfg.rules.iter().any(|rule| {
        let mut base = rule.src.as_path();
        while magic(base) {
            let Some(parent) = base.parent() else {
                return false;
            };
            base = parent;
        }
        let to: PathBuf = rule
            .to
            .components()
            .filter(|component| *component != std::path::Component::CurDir)
            .collect();
        let Ok(relative) = link.backup_path.strip_prefix(&to) else {
            return false;
        };
        let prefix = rule.src.strip_prefix(base).unwrap();
        let selected = rule.targets.iter().any(|target| {
            let pattern = prefix.join(target);
            glob::Pattern::new(&pattern.to_string_lossy()).is_ok_and(|pattern| {
                relative.ancestors().any(|path| {
                    pattern.matches_path_with(
                        path,
                        glob::MatchOptions {
                            case_sensitive: true,
                            require_literal_separator: true,
                            require_literal_leading_dot: false,
                        },
                    )
                })
            })
        });
        // Nested links inside a dereferenced directory originate under its target.
        let location = manifest
            .links
            .iter()
            .filter(|parent| {
                parent.mode == Symlinks::Dereference
                    && parent.backup_path != link.backup_path
                    && link.backup_path.starts_with(&parent.backup_path)
            })
            .max_by_key(|parent| parent.backup_path.components().count())
            .map(|parent| {
                parent
                    .resolved_target
                    .join(link.backup_path.strip_prefix(&parent.backup_path).unwrap())
            })
            .unwrap_or_else(|| base.join(relative));
        // Changing machines/users is handled by the separate home-mapping prompt.
        let location = std::env::var_os("HOME")
            .and_then(|home| {
                location
                    .strip_prefix(Path::new(&home))
                    .ok()
                    .map(|suffix| manifest.backup_home.join(suffix))
            })
            .unwrap_or(location);
        selected
            && location == link.location
            && (rule.symlinks == Some(Symlinks::Prompt) || rule.symlinks == Some(link.mode))
    })
}

fn check_selected(root: &Path, path: &Path, manifest: &Manifest) -> Result<(), String> {
    let key = path.strip_prefix(root).map_err(|e| e.to_string())?;
    let mut parent = root.to_path_buf();
    if let Some(parts) = key.parent() {
        for part in parts.components() {
            parent.push(part);
            if !fs::symlink_metadata(&parent).is_ok_and(|m| m.is_dir() && !m.is_symlink()) {
                return Err(format!(
                    "Backup path has a non-directory or symlink parent: {}",
                    parent.display()
                ));
            }
        }
    }
    let metadata = fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if metadata.is_symlink()
        && !manifest
            .links
            .iter()
            .any(|l| l.backup_path == key && l.mode == Symlinks::Reference)
    {
        return Err(format!(
            "Backup symlink is missing from manifest: {}",
            path.display()
        ));
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            check_selected(root, &entry.map_err(|e| e.to_string())?.path(), manifest)?;
        }
    } else {
        check_entry(path)?;
    }
    Ok(())
}

pub(super) fn manifest(cfg: &Config) -> Result<Manifest, String> {
    let manifest = Manifest::load(&cfg.destination)?
        .ok_or("Missing symlink manifest; create a successful backup before restoring")?;
    let differences: Vec<PathBuf> = manifest
        .links
        .iter()
        .filter(|link| !consistent(cfg, &manifest, link))
        .map(|link| link.backup_path.clone())
        .collect();
    if !differences.is_empty() {
        return Err(format!(
            "Symlink manifest differs from YAML: {}. Fix the configuration or create a new backup before restoring.",
            differences
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    for link in &manifest.links {
        check_link(&cfg.destination, link)
            .map_err(|error| format!("Incomplete symlink manifest: {error}. Fix the backup or create a new backup before restoring."))?;
    }
    for rule in &cfg.rules {
        let mut base = rule.src.as_path();
        while magic(base) {
            base = base.parent().ok_or("Invalid source pattern")?;
        }
        let prefix = rule.src.strip_prefix(base).map_err(|e| e.to_string())?;
        for target in &rule.targets {
            let path = cfg.destination.join(&rule.to).join(prefix).join(target);
            let matches = expand(&path)?;
            if matches.is_empty() {
                return Err(format!(
                    "Backup path selected by YAML is missing: {}. Fix the configuration or backup before restoring.",
                    path.display()
                ));
            }
            for path in matches {
                check_selected(&cfg.destination, &path, &manifest)?;
            }
        }
    }
    Ok(manifest)
}
