use crate::{
    config::{Config, Rule, Symlinks},
    hooks,
    manifest::{Link, Manifest, TargetLink},
    report,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn magic(path: &Path) -> bool {
    path.to_string_lossy().contains(['*', '?', '['])
}
fn remove(path: &Path) -> std::io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}
fn clean_backup(path: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_name() != crate::manifest::NAME && entry.file_name() != ".cockup-incomplete" {
            remove(&entry.path())?;
        }
    }
    Ok(())
}
fn resolved(path: &Path) -> std::io::Result<PathBuf> {
    if path.exists() {
        return fs::canonicalize(path);
    }
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("Path has no parent"))?;
    Ok(resolved(parent)?.join(
        path.file_name()
            .ok_or_else(|| std::io::Error::other("Path has no filename"))?,
    ))
}
fn entry_path(path: &Path) -> std::io::Result<PathBuf> {
    if fs::symlink_metadata(path).is_ok_and(|m| m.is_symlink()) {
        return Ok(resolved(path.parent().unwrap())?.join(path.file_name().unwrap()));
    }
    resolved(path)
}
fn check_overlap(src: &Path, dst: &Path) -> Result<(), String> {
    let source = entry_path(src).map_err(|e| e.to_string())?;
    let target = entry_path(dst).map_err(|e| e.to_string())?;
    if source.starts_with(&target) || target.starts_with(&source) {
        return Err(format!(
            "Overlapping source and destination: {} -> {}",
            src.display(),
            dst.display()
        ));
    }
    Ok(())
}
struct CopyState {
    root: PathBuf,
    manifest: Manifest,
    restore: bool,
    planning: bool,
    planned: Vec<Link>,
    replacing_directories: Vec<(PathBuf, PathBuf)>,
    prompt_choice: Option<Result<Symlinks, String>>,
    home_mapping: Option<(PathBuf, PathBuf)>,
}
impl CopyState {
    fn mapped(&self, path: &Path) -> PathBuf {
        match &self.home_mapping {
            Some((old, new)) => path
                .strip_prefix(old)
                .or_else(|_| path.strip_prefix(&self.manifest.canonical_home))
                .map(|suffix| new.join(suffix))
                .unwrap_or_else(|_| path.to_path_buf()),
            None => path.to_path_buf(),
        }
    }
    fn link_target(&self, location: &Path, target: &Path, destination: &Path) -> PathBuf {
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
    fn choose_home(&mut self, cfg: &Config) -> Result<(), String> {
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
            report::warning(&format!(
                "Restore user paths from {} ({}):\n[c]urrent user ({}) or [o]riginal user?",
                self.manifest.backup_user,
                old.display(),
                current.display()
            ));
            match report::input("")?.to_lowercase().as_str() {
                "c" | "current" => self.home_mapping = Some((old.clone(), current)),
                "o" | "original" => self.home_mapping = Some((current, old.clone())),
                _ => {
                    return Err(
                        "Restore requires an explicit current or original user choice".into(),
                    );
                }
            }
        }
        Ok(())
    }
}
fn copy(
    src: &Path,
    dst: &Path,
    metadata: bool,
    symlinks: Symlinks,
    ancestors: &[PathBuf],
    state: &mut CopyState,
) -> Result<(), String> {
    let key: PathBuf = (if state.restore { src } else { dst })
        .strip_prefix(&state.root)
        .map_err(|e| e.to_string())?
        .components()
        .filter(|c| *c != std::path::Component::CurDir)
        .collect();
    if !state.restore
        && (key.as_os_str().is_empty()
            || key.components().next().is_some_and(|c| {
                let name = c.as_os_str().to_string_lossy();
                name.starts_with(crate::manifest::NAME) || name.starts_with(".cockup-incomplete")
            }))
    {
        return Err("Reserved backup metadata path".into());
    }
    if state.restore
        && let Some(index) = state
            .manifest
            .links
            .iter()
            .position(|l| l.backup_path == key)
    {
        let link = state.manifest.links.remove(index);
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
                for entry in link.target_chain.iter().rev() {
                    let location = state.mapped(&entry.location);
                    fs::create_dir_all(location.parent().unwrap()).map_err(|e| e.to_string())?;
                    remove(&location).map_err(|e| e.to_string())?;
                    std::os::unix::fs::symlink(
                        state.link_target(&entry.location, &entry.target, &location),
                        &location,
                    )
                    .map_err(|e| e.to_string())?;
                }
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
        return result;
    }
    let source_entry = entry_path(src).map_err(|e| e.to_string())?;
    let link = fs::symlink_metadata(src)
        .map_err(|e| e.to_string())?
        .is_symlink();
    let planned = state.planned.iter().find(|l| l.backup_path == key);
    let choice = if link
        && !state.planning
        && !state.restore
        && let Some(planned) = planned
    {
        planned.mode
    } else if link && symlinks == Symlinks::Prompt {
        if state.prompt_choice.is_none() {
            report::warning(&format!(
                "Symlink {} -> {}:\n[r]eferece link or [d]ereference target? (applies to all symlinks)",
                src.display(),
                fs::read_link(src).map_err(|e| e.to_string())?.display()
            ));
            state.prompt_choice =
                Some(
                    report::input("").and_then(|answer| match answer.to_lowercase().as_str() {
                        "r" | "referece" => Ok(Symlinks::Referece),
                        "d" | "dereference" => Ok(Symlinks::Dereference),
                        _ => Err(
                            "Symlinks require an explicit referece or dereference choice".into(),
                        ),
                    }),
                );
        }
        state.prompt_choice.as_ref().unwrap().clone()?
    } else {
        symlinks
    };
    let dereferenced = if link && choice == Symlinks::Dereference {
        Some(
            fs::canonicalize(src)
                .map_err(|e| format!("Cannot dereference {}: {e}", src.display()))?,
        )
    } else {
        None
    };
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
            backup_path: key.clone(),
            target_chain,
        })
    } else {
        None
    };
    let src = dereferenced.as_deref().unwrap_or(src);
    let source = entry_path(src).map_err(|e| e.to_string())?;
    // Children will be written into a new directory, not through its old links.
    let target = if let Some((directory, identity)) = state.replacing_directories.last() {
        identity.join(dst.strip_prefix(directory).map_err(|e| e.to_string())?)
    } else {
        entry_path(dst).map_err(|e| e.to_string())?
    };
    if source_entry == target || source.starts_with(&target) || target.starts_with(&source) {
        return Err(format!(
            "Overlapping source and destination: {} -> {}",
            src.display(),
            dst.display()
        ));
    }
    let mut ancestors = ancestors.to_vec();
    if fs::symlink_metadata(src)
        .map_err(|e| e.to_string())?
        .is_dir()
    {
        if ancestors.contains(&source) {
            return Err(format!("Symlink cycle at {}", src.display()));
        }
        ancestors.push(source);
    }
    if state.planning {
        if let Some(record) = record {
            state.planned.push(record);
        }
        if fs::symlink_metadata(src)
            .map_err(|e| e.to_string())?
            .is_dir()
        {
            let entries = fs::read_dir(src).map_err(|e| e.to_string())?;
            state
                .replacing_directories
                .push((dst.to_path_buf(), target));
            let mut errors = Vec::new();
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        if let Err(error) = copy(
                            &entry.path(),
                            &dst.join(entry.file_name()),
                            metadata,
                            symlinks,
                            &ancestors,
                            state,
                        ) {
                            errors.push(error);
                        }
                    }
                    Err(error) => errors.push(error.to_string()),
                }
            }
            state.replacing_directories.pop();
            if !errors.is_empty() {
                return Err(errors.join("; "));
            }
        }
        return Ok(());
    }
    fn inner(
        src: &Path,
        dst: &Path,
        preserve: bool,
        symlinks: Symlinks,
        ancestors: &[PathBuf],
        state: &mut CopyState,
    ) -> std::io::Result<()> {
        let meta = fs::symlink_metadata(src)?;
        if !meta.is_file() && !meta.is_dir() && !meta.is_symlink() {
            report::warning(&format!("Skipping non-regular file: {}", src.display()));
            return Ok(());
        }
        remove(dst)?;
        fs::create_dir_all(
            dst.parent()
                .ok_or_else(|| std::io::Error::other("Destination has no parent"))?,
        )?;
        if meta.is_symlink() {
            std::os::unix::fs::symlink(state.link_target(src, &fs::read_link(src)?, dst), dst)?;
        } else if meta.is_dir() {
            fs::create_dir(dst)?;
            let mut errors = Vec::new();
            for entry in fs::read_dir(src)? {
                match entry {
                    Ok(entry) => {
                        if let Err(e) = copy(
                            &entry.path(),
                            &dst.join(entry.file_name()),
                            preserve,
                            symlinks,
                            ancestors,
                            state,
                        ) {
                            errors.push(e);
                        }
                    }
                    Err(e) => errors.push(e.to_string()),
                }
            }
            if !errors.is_empty() {
                return Err(std::io::Error::other(errors.join("; ")));
            }
        } else {
            fs::copy(src, dst)?;
            fs::set_permissions(dst, meta.permissions())?;
            if !preserve {
                let now = filetime::FileTime::now();
                filetime::set_file_times(dst, now, now)?;
            }
        }
        // Match copy2: preserve regular-file and link metadata, not directory metadata.
        if preserve && !meta.is_dir() {
            let atime = filetime::FileTime::from_last_access_time(&meta);
            let mtime = filetime::FileTime::from_last_modification_time(&meta);
            if meta.is_symlink() {
                filetime::set_symlink_file_times(dst, atime, mtime)?;
            } else {
                filetime::set_file_times(dst, atime, mtime)?;
            }
            preserve_flags(dst, &meta)?;
        }
        Ok(())
    }
    let updating = fs::symlink_metadata(dst).is_ok();
    inner(src, dst, metadata, symlinks, &ancestors, state)
        .map_err(|e| format!("{} -> {}: {e}", src.display(), dst.display()))?;
    if !state.restore {
        state.manifest.links.retain(|l| l.backup_path != key);
        if let Some(record) = record {
            state.manifest.links.push(record);
        }
    }
    report::success(&format!(
        "{}: {}",
        if updating { "Updated" } else { "Copied" },
        src.display()
    ));
    Ok(())
}
#[cfg(target_os = "macos")]
fn preserve_flags(path: &Path, metadata: &fs::Metadata) -> std::io::Result<()> {
    use std::{
        ffi::CString,
        os::{macos::fs::MetadataExt, unix::ffi::OsStrExt},
    };
    unsafe extern "C" {
        fn lchflags(path: *const std::ffi::c_char, flags: std::ffi::c_uint) -> std::ffi::c_int;
    }
    let path = CString::new(path.as_os_str().as_bytes())?;
    // SAFETY: path is a valid NUL-terminated C string and lchflags does not retain it.
    if unsafe { lchflags(path.as_ptr(), metadata.st_flags()) } == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
#[cfg(not(target_os = "macos"))]
fn preserve_flags(_: &Path, _: &fs::Metadata) -> std::io::Result<()> {
    Ok(())
}
fn rule(
    rule: &Rule,
    destination: &Path,
    restore: bool,
    metadata: bool,
    symlinks: Symlinks,
    state: &mut CopyState,
) -> usize {
    let mut base = rule.src.clone();
    while magic(&base) {
        base = base.parent().unwrap().to_path_buf();
    }
    let prefix = rule.src.strip_prefix(&base).unwrap();
    let backup = destination.join(&rule.to);
    let (from, to) = if restore {
        (&backup, &base)
    } else {
        (&base, &backup)
    };
    let mut failures = 0;
    for target in &rule.targets {
        let target = prefix.join(target);
        let source = from.join(target);
        let paths: Result<Vec<PathBuf>, String> = if fs::symlink_metadata(&source).is_ok() {
            Ok(vec![source.clone()])
        } else if magic(&source) {
            glob::glob_with(
                &source.to_string_lossy(),
                glob::MatchOptions {
                    case_sensitive: true,
                    require_literal_separator: true,
                    require_literal_leading_dot: false,
                },
            )
            .map_err(|e| e.to_string())
            .and_then(|paths| {
                paths
                    .filter(|p| {
                        p.as_ref().map_or(true, |p| {
                            let last = p
                                .as_os_str()
                                .as_encoded_bytes()
                                .rsplit(|c| *c == b'/')
                                .next()
                                .unwrap_or_default();
                            last != b"."
                                && last != b".."
                                && glob::Pattern::new(&source.to_string_lossy()).is_ok_and(
                                    |pattern| {
                                        pattern.matches_path_with(
                                            p,
                                            glob::MatchOptions {
                                                case_sensitive: true,
                                                require_literal_separator: true,
                                                require_literal_leading_dot: true,
                                            },
                                        )
                                    },
                                )
                        })
                    })
                    .map(|p| p.map_err(|e| e.to_string()))
                    .collect()
            })
        } else {
            report::warning(&format!("Source not found, skipping: {}", source.display()));
            Ok(vec![])
        };
        match paths {
            Err(e) => {
                report::error(&e);
                failures += 1;
            }
            Ok(paths) => {
                if paths.is_empty() && magic(&source) {
                    report::error(&format!(
                        "Matches not found for pattern: {}",
                        source.display()
                    ));
                    failures += 1;
                }
                for src in paths {
                    let relative = src.strip_prefix(from).unwrap();
                    if let Err(e) = copy(
                        &src,
                        &if restore {
                            state.mapped(&to.join(relative))
                        } else {
                            to.join(relative)
                        },
                        metadata,
                        symlinks,
                        &[],
                        state,
                    ) {
                        report::error(&e);
                        failures += 1;
                    }
                }
            }
        }
    }
    failures
}
pub fn execute(cfg: &Config, restore: bool) -> Result<(), String> {
    let incomplete = cfg.destination.join(".cockup-incomplete");
    if restore && fs::symlink_metadata(&incomplete).is_ok() {
        return Err("Backup is incomplete; run a successful backup before restoring".into());
    }
    let stored = Manifest::load(&cfg.destination)?;
    if restore && stored.is_none() && cfg.symlinks != Symlinks::Referece {
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
        planned: Vec::new(),
        replacing_directories: Vec::new(),
        prompt_choice: None,
        home_mapping: None,
    };
    if restore {
        state.choose_home(cfg)?;
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
    if !restore {
        state.planning = true;
        let mut planning_failures = 0;
        for rule_config in &cfg.rules {
            planning_failures += rule(
                rule_config,
                &cfg.destination,
                false,
                cfg.metadata,
                cfg.symlinks,
                &mut state,
            );
        }
        state.planning = false;
        if planning_failures > 0 && (cfg.clean || cfg.symlinks == Symlinks::Prompt) {
            return Err("Symlink planning failed; backup destination was not modified".into());
        }
    }
    let mut failures = hooks::run(
        if restore {
            &cfg.hooks.pre_restore
        } else {
            &cfg.hooks.pre_backup
        },
        &cfg.directory,
    );
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
            clean_backup(&cfg.destination).map_err(|e| e.to_string())?;
        }
        fs::create_dir_all(&cfg.destination).map_err(|e| e.to_string())?;
    }
    for r in &cfg.rules {
        failures += hooks::run(&r.on_start, &cfg.destination);
        failures += rule(
            r,
            &cfg.destination,
            restore,
            cfg.metadata,
            if restore {
                Symlinks::Referece
            } else {
                cfg.symlinks
            },
            &mut state,
        );
        failures += hooks::run(&r.on_end, &cfg.destination);
    }
    failures += hooks::run(
        if restore {
            &cfg.hooks.post_restore
        } else {
            &cfg.hooks.post_backup
        },
        &cfg.destination,
    );
    if failures > 0 {
        return Err(format!("Operation completed with {failures} failures."));
    }
    if !restore {
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
