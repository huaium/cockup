use super::{
    paths::{entry_path, preserve_flags, remove},
    state::CopyState,
    symlinks,
};
use crate::{config::Symlinks, report};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn copy(
    src: &Path,
    dst: &Path,
    metadata: bool,
    symlinks: Symlinks,
    ancestors: &[PathBuf],
    state: &mut CopyState,
) -> Result<(), String> {
    let top_level = ancestors.is_empty();
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
        && let Some(result) = symlinks::restore_link(src, dst, metadata, ancestors, state, &key)
    {
        return result;
    }
    let source_entry = entry_path(src).map_err(|e| e.to_string())?;
    let link = fs::symlink_metadata(src)
        .map_err(|e| e.to_string())?
        .is_symlink();
    let choice = symlinks::choose_mode(src, &key, symlinks, link, state)?;
    let dereferenced = if link && choice == Symlinks::Dereference {
        Some(
            fs::canonicalize(src)
                .map_err(|e| format!("Cannot dereference {}: {e}", src.display()))?,
        )
    } else {
        None
    };
    let record = symlinks::record_link(src, &key, choice, link, state)?;
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
        if state.dry_run {
            let meta = fs::symlink_metadata(src).map_err(|e| e.to_string())?;
            if !meta.is_file() && !meta.is_dir() && !meta.is_symlink() {
                report::warning(&format!(
                    "Skipping non-regular file: {}",
                    report::display_path(src)
                ));
                return Ok(());
            }
            let action = if fs::symlink_metadata(dst).is_ok() && (state.restore || !state.clean) {
                "Would replace"
            } else {
                "Would copy"
            };
            let kind = if link && choice == Symlinks::Reference {
                "symlink"
            } else if meta.is_dir() {
                "folder"
            } else {
                "file"
            };
            println!("{action} {kind}: {} -> {}", src.display(), dst.display());
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
    let source_metadata = fs::symlink_metadata(src).map_err(|e| e.to_string())?;
    if !source_metadata.is_file() && !source_metadata.is_dir() && !source_metadata.is_symlink() {
        use std::os::unix::fs::MetadataExt;
        report::warning(&format!(
            "Skipping non-regular file: {} (0o{:o})",
            report::display_path(src),
            source_metadata.mode()
        ));
        return Ok(());
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
    if top_level {
        let kind = if source_metadata.is_symlink() {
            "Symlink"
        } else if source_metadata.is_dir() {
            "Folder"
        } else {
            "File"
        };
        report::copied(kind, updating, src);
    }
    Ok(())
}
