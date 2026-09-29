use crate::{
    config::{Config, Symlinks},
    manifest::{Link, Manifest},
    report,
};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RestorePolicy {
    Ask,
    Override,
    SkipExisting,
}

pub(super) struct CopyState {
    pub(super) root: PathBuf,
    pub(super) manifest: Manifest,
    pub(super) restore: bool,
    pub(super) restore_policy: RestorePolicy,
    pub(super) planning: bool,
    pub(super) dry_run: bool,
    pub(super) clean: bool,
    pub(super) planned: Vec<Link>,
    pub(super) replacing_directories: Vec<(PathBuf, PathBuf)>,
    pub(super) prompt_choice: Option<Result<Symlinks, String>>,
    pub(super) home_mapping: Option<(PathBuf, PathBuf)>,
}
impl CopyState {
    pub(super) fn mapped(&self, path: &Path) -> PathBuf {
        match &self.home_mapping {
            Some((old, new)) => path
                .strip_prefix(old)
                .or_else(|_| path.strip_prefix(&self.manifest.canonical_home))
                .map(|suffix| new.join(suffix))
                .unwrap_or_else(|_| path.to_path_buf()),
            None => path.to_path_buf(),
        }
    }
    pub(super) fn link_target(
        &self,
        location: &Path,
        target: &Path,
        destination: &Path,
    ) -> PathBuf {
        if target.is_absolute() {
            return self.mapped(target);
        }
        let parent = destination.parent().unwrap();
        let original_parent = location.parent().unwrap();
        let mut original = original_parent.to_path_buf();
        let mut remainder = target.components().peekable();
        // Only leading parent components can be used for home-prefix mapping.
        // Never collapse a/..: a may itself be a symlink.
        while let Some(component) = remainder.peek() {
            match component {
                std::path::Component::ParentDir if self.home_mapping.is_some() => {
                    original.pop();
                    remainder.next();
                }
                std::path::Component::CurDir => {
                    remainder.next();
                }
                _ => break,
            }
        }
        let suffix: PathBuf = remainder.collect();
        let (mapped, suffix) = if self.home_mapping.is_some() {
            (self.mapped(&original.join(&suffix)), PathBuf::new())
        } else {
            (original, suffix)
        };
        if parent == original_parent
            && (self.home_mapping.is_none()
                || self.mapped(&original_parent.join(target)) == original_parent.join(target))
        {
            return target.to_path_buf();
        }
        let from: Vec<_> = parent.components().collect();
        let to: Vec<_> = mapped.components().collect();
        let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
        let mut relative = PathBuf::new();
        for _ in common..from.len() {
            relative.push("..");
        }
        for c in &to[common..] {
            relative.push(c);
        }
        if !suffix.as_os_str().is_empty() {
            relative.push(suffix);
        }
        if relative.as_os_str().is_empty() {
            relative.push(".");
        }
        relative
    }
    pub(super) fn choose_home(&mut self, cfg: &Config, action: &str) -> Result<(), String> {
        let current = PathBuf::from(std::env::var_os("HOME").ok_or("HOME is not set")?);
        let old = &self.manifest.backup_home;
        let affected = cfg.rules.iter().any(|r| {
            r.src.starts_with(old)
                || r.src.starts_with(&self.manifest.canonical_home)
                || r.src.starts_with(&current)
        }) || self.manifest.links.iter().any(|l| {
            l.location.starts_with(old)
                || l.resolved_target.starts_with(old)
                || l.target.starts_with(old)
        });
        if affected
            && (current != *old
                || std::env::var("USER").unwrap_or_default() != self.manifest.backup_user)
        {
            let prompt = format!(
                "{action} user paths from {} ({}):\n[c]urrent user ({}) or [o]riginal user?",
                self.manifest.backup_user,
                old.display(),
                current.display()
            );
            loop {
                report::warning(&prompt);
                match report::input("")?.to_lowercase().as_str() {
                    "c" | "current" => {
                        self.home_mapping = Some((old.clone(), current));
                        break;
                    }
                    "o" | "original" => {
                        self.home_mapping = Some((current, old.clone()));
                        break;
                    }
                    _ => report::warning("Please enter c or o."),
                }
            }
        }
        Ok(())
    }
}
