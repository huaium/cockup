use super::{
    paths::magic,
    rules::expand,
    state::{CopyState, RestorePolicy},
};
use crate::{
    config::{Config, Symlinks},
    manifest::Manifest,
    report,
};
use std::{
    collections::BTreeSet,
    fs,
    io::{BufReader, Read},
    path::Path,
};

const TEXT_LIMIT: u64 = 1024 * 1024;

fn label(path: &Path) -> String {
    std::env::current_dir()
        .ok()
        .and_then(|cwd| path.strip_prefix(cwd).ok().map(|p| p.display().to_string()))
        .unwrap_or_else(|| report::display_path(path))
}

pub(super) fn show_text_diff(saved: &Path, local: &Path, missing: bool) -> Result<(), String> {
    let saved_len = fs::metadata(saved)
        .map_err(|e| format!("{}: {e}", saved.display()))?
        .len();
    let local_len = if missing {
        0
    } else {
        fs::metadata(local)
            .map_err(|e| format!("{}: {e}", local.display()))?
            .len()
    };
    if saved_len > TEXT_LIMIT || local_len > TEXT_LIMIT {
        println!("  Content diff omitted (file exceeds 1 MiB).");
        return Ok(());
    }
    let old = fs::read(saved).map_err(|e| format!("{}: {e}", saved.display()))?;
    let new = if missing {
        Vec::new()
    } else {
        fs::read(local).map_err(|e| format!("{}: {e}", local.display()))?
    };
    if old.contains(&0) || new.contains(&0) {
        println!("  Binary files differ.");
        return Ok(());
    }
    let (Ok(old), Ok(new)) = (std::str::from_utf8(&old), std::str::from_utf8(&new)) else {
        println!("  Non-UTF-8 files differ.");
        return Ok(());
    };
    let diff = similar::TextDiff::from_lines(old, new);
    let rendered = diff
        .unified_diff()
        .header(&label(saved), &label(local))
        .to_string();
    report::diff_lines(&rendered);
    Ok(())
}

fn show_added_file(path: &Path) -> Result<(), String> {
    let length = fs::metadata(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .len();
    if length > TEXT_LIMIT {
        println!("  Content diff omitted (file exceeds 1 MiB).");
        return Ok(());
    }
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.contains(&0) {
        println!("  Binary file exists only locally.");
    } else if let Ok(content) = std::str::from_utf8(&bytes) {
        let diff = similar::TextDiff::from_lines("", content);
        report::diff_lines(
            &diff
                .unified_diff()
                .header("/dev/null", &label(path))
                .to_string(),
        );
    } else {
        println!("  Non-UTF-8 file exists only locally.");
    }
    Ok(())
}

fn metadata(path: &Path) -> Result<Option<fs::Metadata>, String> {
    match fs::symlink_metadata(path) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn changed(kind: &str, path: &Path) -> usize {
    println!("{kind}: {}", report::display_path(path));
    1
}

pub(super) fn same_contents(left: &Path, right: &Path) -> Result<bool, String> {
    let length = fs::metadata(left)
        .map_err(|error| format!("{}: {error}", left.display()))?
        .len();
    if length
        != fs::metadata(right)
            .map_err(|error| format!("{}: {error}", right.display()))?
            .len()
    {
        return Ok(false);
    }
    let open = |path: &Path| {
        fs::File::open(path)
            .map(BufReader::new)
            .map_err(|error| format!("{}: {error}", path.display()))
    };
    let mut left = open(left)?;
    let mut right = open(right)?;
    let mut first = [0; 8192];
    let mut second = [0; 8192];
    let mut remaining = length;
    while remaining > 0 {
        let count = remaining.min(first.len() as u64) as usize;
        left.read_exact(&mut first[..count])
            .map_err(|error| error.to_string())?;
        right
            .read_exact(&mut second[..count])
            .map_err(|error| error.to_string())?;
        if first[..count] != second[..count] {
            return Ok(false);
        }
        remaining -= count as u64;
    }
    Ok(true)
}

fn compare_link(path: &Path, target: &Path, summary: bool) -> Result<usize, String> {
    match metadata(path)? {
        None => {
            let count = changed("Missing locally", path);
            if !summary {
                report::diff_lines(&format!("-link -> {}\n+missing\n", target.display()));
            }
            Ok(count)
        }
        Some(info)
            if info.is_symlink() && fs::read_link(path).map_err(|e| e.to_string())? == target =>
        {
            Ok(0)
        }
        Some(info) => {
            let count = changed("Changed", path);
            if !summary {
                let current = if info.is_symlink() {
                    format!(
                        "link -> {}",
                        fs::read_link(path).map_err(|e| e.to_string())?.display()
                    )
                } else {
                    "not a symlink".into()
                };
                report::diff_lines(&format!("-link -> {}\n+{current}\n", target.display()));
            }
            Ok(count)
        }
    }
}

fn compare_plain(
    backup: &Path,
    current: &Path,
    state: &CopyState,
    summary: bool,
) -> Result<usize, String> {
    let saved =
        metadata(backup)?.ok_or_else(|| format!("Backup path not found: {}", backup.display()))?;
    let Some(local) = metadata(current)? else {
        let count = changed("Missing locally", current);
        if !summary && saved.is_file() {
            show_text_diff(backup, current, true)?;
        }
        return Ok(count);
    };
    if saved.is_file() && local.is_file() {
        return if same_contents(backup, current)? {
            Ok(0)
        } else {
            let count = changed("Changed", current);
            if !summary {
                show_text_diff(backup, current, false)?;
            }
            Ok(count)
        };
    }
    if saved.is_symlink() && local.is_symlink() {
        return compare_link(
            current,
            &fs::read_link(backup).map_err(|e| e.to_string())?,
            summary,
        );
    }
    if saved.is_dir() && local.is_dir() {
        let mut names = BTreeSet::new();
        for directory in [backup, current] {
            for entry in
                fs::read_dir(directory).map_err(|e| format!("{}: {e}", directory.display()))?
            {
                let entry = entry.map_err(|e| e.to_string())?;
                names.insert(entry.file_name());
            }
        }
        let mut changes = 0;
        for name in names {
            let saved_child = backup.join(&name);
            let local_child = current.join(name);
            if metadata(&saved_child)?.is_none() {
                changes += changed("Only locally", &local_child);
                if !summary && metadata(&local_child)?.is_some_and(|info| info.is_file()) {
                    show_added_file(&local_child)?;
                }
            } else {
                changes += compare_entry(&saved_child, &local_child, state, summary)?;
            }
        }
        return Ok(changes);
    }
    Ok(changed("Changed type", current))
}

fn compare_entry(
    backup: &Path,
    current: &Path,
    state: &CopyState,
    summary: bool,
) -> Result<usize, String> {
    let key = backup
        .strip_prefix(&state.root)
        .map_err(|e| e.to_string())?;
    let Some(link) = state
        .manifest
        .links
        .iter()
        .find(|entry| entry.backup_path == key)
    else {
        return compare_plain(backup, current, state, summary);
    };
    let saved =
        metadata(backup)?.ok_or_else(|| format!("Backup path not found: {}", backup.display()))?;
    let valid = match link.mode {
        Symlinks::Reference => saved.is_symlink(),
        Symlinks::Dereference => saved.is_file() || saved.is_dir(),
        Symlinks::Prompt => false,
    };
    if !valid {
        return Err(format!("Backup entry has wrong type: {}", backup.display()));
    }
    let expected = state.link_target(&link.location, &link.target, current);
    let mut changes = compare_link(current, &expected, summary)?;
    if link.mode == Symlinks::Dereference {
        changes += compare_plain(backup, &state.mapped(&link.resolved_target), state, summary)?;
        for entry in &link.target_chain {
            let location = state.mapped(&entry.location);
            let target = state.link_target(&entry.location, &entry.target, &location);
            changes += compare_link(&location, &target, summary)?;
        }
    }
    Ok(changes)
}

pub(crate) fn run(cfg: &Config, summary: bool) -> Result<(), String> {
    if !cfg.destination.is_dir() {
        return Err(format!(
            "Backup destination is not a directory: {}",
            cfg.destination.display()
        ));
    }
    if metadata(&cfg.destination.join(".cockup-incomplete"))?.is_some() {
        return Err("Backup is incomplete; run a successful backup before comparing".into());
    }
    let manifest = Manifest::load(&cfg.destination)?
        .ok_or_else(|| format!("Missing symlink manifest: {}", cfg.destination.display()))?;
    let mut state = CopyState {
        root: cfg.destination.clone(),
        manifest,
        restore: true,
        restore_policy: RestorePolicy::Ask,
        planning: false,
        dry_run: true,
        clean: cfg.clean,
        planned: Vec::new(),
        replacing_directories: Vec::new(),
        prompt_choice: None,
        home_mapping: None,
    };
    state.choose_home(cfg, "Compare")?;
    let mut changes = 0;
    let mut failures = 0;
    for rule in &cfg.rules {
        let mut base = rule.src.clone();
        while magic(&base) {
            base = base.parent().unwrap().to_path_buf();
        }
        let prefix = rule.src.strip_prefix(&base).unwrap();
        let backup_root = cfg.destination.join(&rule.to);
        for target in &rule.targets {
            let source = backup_root.join(prefix.join(target));
            match expand(&source) {
                Ok(paths) if paths.is_empty() => {
                    report::error(&format!("Backup path not found: {}", source.display()));
                    failures += 1;
                }
                Ok(paths) => {
                    for path in paths {
                        let relative = path.strip_prefix(&backup_root).unwrap();
                        let current = state.mapped(&base.join(relative));
                        match compare_entry(&path, &current, &state, summary) {
                            Ok(count) => changes += count,
                            Err(error) => {
                                report::error(&error);
                                failures += 1;
                            }
                        }
                    }
                }
                Err(error) => {
                    report::error(&error);
                    failures += 1;
                }
            }
        }
    }
    if failures > 0 {
        return Err(format!("Comparison failed: {failures} problems found."));
    }
    if changes == 0 {
        report::success("No differences found.");
    } else {
        println!("{changes} differences found.");
    }
    Ok(())
}
