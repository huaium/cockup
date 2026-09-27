use crate::{
    config::{Config, Rule},
    hooks, report,
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
fn copy(src: &Path, dst: &Path, metadata: bool) -> Result<(), String> {
    let source = resolved(src).map_err(|e| e.to_string())?;
    let target = resolved(dst).map_err(|e| e.to_string())?;
    if source.starts_with(&target) || target.starts_with(&source) {
        return Err(format!(
            "Overlapping source and destination: {} -> {}",
            src.display(),
            dst.display()
        ));
    }
    fn inner(src: &Path, dst: &Path, preserve: bool) -> std::io::Result<()> {
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
            std::os::unix::fs::symlink(fs::read_link(src)?, dst)?;
        } else if meta.is_dir() {
            fs::create_dir(dst)?;
            let mut errors = Vec::new();
            for entry in fs::read_dir(src)? {
                match entry {
                    Ok(entry) => {
                        if let Err(e) = copy(&entry.path(), &dst.join(entry.file_name()), preserve)
                        {
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
    inner(src, dst, metadata)
        .map_err(|e| format!("{} -> {}: {e}", src.display(), dst.display()))?;
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
pub fn rule(rule: &Rule, destination: &Path, restore: bool, metadata: bool) -> usize {
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
        let source = from.join(&target);
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
                    if let Err(e) = copy(&src, &to.join(relative), metadata) {
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
    let mut failures = hooks::run(
        if restore {
            &cfg.hooks.pre_restore
        } else {
            &cfg.hooks.pre_backup
        },
        &cfg.directory,
    );
    if restore {
        if !cfg.destination.is_dir() {
            return Err("Backup destination is not a directory".into());
        }
    } else {
        if cfg.clean {
            remove(&cfg.destination).map_err(|e| e.to_string())?;
        }
        fs::create_dir_all(&cfg.destination).map_err(|e| e.to_string())?;
    }
    for r in &cfg.rules {
        failures += hooks::run(&r.on_start, &cfg.destination);
        failures += rule(r, &cfg.destination, restore, cfg.metadata);
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
    report::success(if restore {
        "Restore completed."
    } else {
        "Backup completed."
    });
    Ok(())
}
