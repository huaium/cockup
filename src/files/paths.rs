use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn magic(path: &Path) -> bool {
    path.to_string_lossy().contains(['*', '?', '['])
}
pub(super) fn remove(path: &Path) -> std::io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}
pub(super) fn clean_backup(path: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_name() != crate::manifest::NAME && entry.file_name() != ".cockup-incomplete" {
            remove(&entry.path())?;
        }
    }
    Ok(())
}
pub(super) fn resolved(path: &Path) -> std::io::Result<PathBuf> {
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
pub(super) fn entry_path(path: &Path) -> std::io::Result<PathBuf> {
    if fs::symlink_metadata(path).is_ok_and(|m| m.is_symlink()) {
        return Ok(resolved(path.parent().unwrap())?.join(path.file_name().unwrap()));
    }
    resolved(path)
}
pub(super) fn check_overlap(src: &Path, dst: &Path) -> Result<(), String> {
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
#[cfg(target_os = "macos")]
pub(super) fn preserve_flags(path: &Path, metadata: &fs::Metadata) -> std::io::Result<()> {
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
pub(super) fn preserve_flags(_: &Path, _: &fs::Metadata) -> std::io::Result<()> {
    Ok(())
}
