use super::*;

#[test]
fn backup_restore_preserves_nested_include_paths_and_glob_layout() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "configs/child/source/app/settings.txt", "included");
    write(p, "configs/source/top.txt", "top");
    write(
        p,
        "configs/child/deep/grand.yaml",
        "symlinks: referece\ndestination: ignored\nrules: []\n",
    );
    write(
        p,
        "configs/child/child.yaml",
        "symlinks: referece\ndestination: ignored\ninclude: [{file: deep/grand.yaml}]\nrules:\n  - from: source/*\n    targets: ['*.txt']\n    to: child\n",
    );
    write(
        p,
        "configs/root.yaml",
        "symlinks: referece\ndestination: ../backup\ninclude: [{file: child/child.yaml}]\nrules:\n  - from: source\n    targets: [top.txt]\n    to: top\n",
    );
    let out = cli(p, &["backup", "configs/root.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(p.join("backup/child/app/settings.txt")).unwrap(),
        "included"
    );
    assert_eq!(
        fs::read_to_string(p.join("backup/top/top.txt")).unwrap(),
        "top"
    );
    fs::remove_dir_all(p.join("configs/child/source")).unwrap();
    fs::remove_dir_all(p.join("configs/source")).unwrap();
    let out = cli(p, &["restore", "configs/root.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(p.join("configs/child/source/app/settings.txt")).unwrap(),
        "included"
    );
    assert_eq!(
        fs::read_to_string(p.join("configs/source/top.txt")).unwrap(),
        "top"
    );
}

#[test]
fn directory_updates_preserve_links_and_metadata_and_skip_sockets() {
    use std::os::unix::{
        fs::{PermissionsExt, symlink},
        net::UnixListener,
    };
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/folder/file", "data");
    let f = p.join("source/folder/file");
    fs::set_permissions(&f, fs::Permissions::from_mode(0o640)).unwrap();
    filetime::set_file_mtime(&f, filetime::FileTime::from_unix_time(1_000_000, 0)).unwrap();
    symlink("file", p.join("source/folder/link")).unwrap();
    symlink("missing", p.join("source/broken")).unwrap();
    let _socket = UnixListener::bind(p.join("source/folder/socket")).unwrap();
    write(p, "backup/files/folder/stale", "stale");
    write(p, "backup/unrelated", "keep");
    write(
        p,
        "config.yaml",
        "symlinks: referece\ndestination: backup\nrules:\n  - from: source\n    targets: [folder, broken]\n    to: files\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(p.join("backup/files/folder/file")).unwrap(),
        "data"
    );
    assert!(!p.join("backup/files/folder/stale").exists());
    assert!(p.join("backup/unrelated").exists());
    assert_eq!(
        fs::read_link(p.join("backup/files/folder/link")).unwrap(),
        Path::new("../../../source/folder/file")
    );
    assert_eq!(
        fs::read_link(p.join("backup/files/broken")).unwrap(),
        Path::new("../../source/missing")
    );
    let meta = fs::metadata(p.join("backup/files/folder/file")).unwrap();
    assert_eq!(meta.permissions().mode() & 0o777, 0o640);
    assert_eq!(
        filetime::FileTime::from_last_modification_time(&meta).unix_seconds(),
        1_000_000
    );
    assert!(!p.join("backup/files/folder/socket").exists());
    assert!(text(&out).contains("Skipping non-regular file"));
    assert!(text(&out).contains("(0o14"));
    let yaml = fs::read_to_string(p.join("config.yaml")).unwrap();
    write(p, "config.yaml", &(yaml + "clean: true\nmetadata: false\n"));
    assert!(
        cli(p, &["backup", "config.yaml", "-q"], "")
            .status
            .success()
    );
    assert!(!p.join("backup/unrelated").exists());
    let meta = fs::metadata(p.join("backup/files/folder/file")).unwrap();
    assert_ne!(
        filetime::FileTime::from_last_modification_time(&meta).unix_seconds(),
        1_000_000
    );
}

#[test]
fn overlapping_copy_is_rejected_without_destroying_source() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/file", "irreplaceable");
    write(
        p,
        "config.yaml",
        "symlinks: referece\ndestination: source\nclean: true\nrules:\n  - from: source\n    targets: [file]\n    to: .\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        fs::read_to_string(p.join("source/file")).unwrap(),
        "irreplaceable"
    );
}

#[test]
fn copy_failure_does_not_prevent_later_targets() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/sub/file", "first");
    write(p, "source/good", "second");
    write(p, "backup/files/sub", "blocks-directory");
    write(
        p,
        "config.yaml",
        "symlinks: referece\ndestination: backup\nrules:\n  - from: source\n    targets: [sub/file, good]\n    to: files\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        fs::read_to_string(p.join("backup/files/good")).unwrap(),
        "second"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn macos_file_flags_are_preserved_with_metadata() {
    use std::os::macos::fs::MetadataExt;
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/file", "flags");
    assert!(
        Command::new("chflags")
            .arg("nodump")
            .arg(p.join("source/file"))
            .status()
            .unwrap()
            .success()
    );
    write(
        p,
        "config.yaml",
        "symlinks: referece\ndestination: backup\nrules:\n  - from: source\n    targets: [file]\n    to: files\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::metadata(p.join("backup/files/file"))
            .unwrap()
            .st_flags()
            & 1,
        1
    );
}
