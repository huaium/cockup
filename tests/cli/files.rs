use super::*;

#[test]
fn restore_override_and_skip_existing_apply_without_prompts() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "source/folder/existing", "saved\n");
    write(root, "source/folder/missing", "restored\n");
    write(
        root,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [folder]\n    to: files\n",
    );
    assert!(
        cli(root, &["backup", "config.yaml", "-a"], "")
            .status
            .success()
    );
    write(root, "source/folder/existing", "local\n");
    fs::remove_file(root.join("source/folder/missing")).unwrap();
    let skipped = cli(root, &["restore", "config.yaml", "-s"], "");
    assert!(skipped.status.success(), "{}", text(&skipped));
    assert!(!text(&skipped).contains("Overwrite local file?"));
    assert_eq!(
        fs::read_to_string(root.join("source/folder/existing")).unwrap(),
        "local\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("source/folder/missing")).unwrap(),
        "restored\n"
    );

    let overridden = cli(root, &["restore", "config.yaml", "-o"], "");
    assert!(overridden.status.success(), "{}", text(&overridden));
    assert!(!text(&overridden).contains("Overwrite local file?"));
    assert_eq!(
        fs::read_to_string(root.join("source/folder/existing")).unwrap(),
        "saved\n"
    );
    assert_eq!(
        cli(root, &["restore", "config.yaml", "-o", "-s"], "")
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        cli(root, &["backup", "config.yaml", "-o"], "")
            .status
            .code(),
        Some(2)
    );
    let help = cli(root, &["restore", "--help"], "");
    assert!(text(&help).contains("--override"));
    assert!(text(&help).contains("--skip-existing"));
}

#[test]
fn restore_skip_existing_dry_run_previews_kept_and_missing_files() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "source/folder/existing", "saved");
    write(root, "source/folder/missing", "saved");
    write(
        root,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [folder]\n    to: files\n",
    );
    assert!(
        cli(root, &["backup", "config.yaml", "-a"], "")
            .status
            .success()
    );
    fs::remove_file(root.join("source/folder/missing")).unwrap();
    let preview = cli(root, &["restore", "config.yaml", "-s", "--dry-run"], "");
    assert!(preview.status.success(), "{}", text(&preview));
    assert!(
        text(&preview).contains("Would keep file:"),
        "{}",
        text(&preview)
    );
    assert!(
        text(&preview).contains("Would copy file:"),
        "{}",
        text(&preview)
    );
    assert!(!root.join("source/folder/missing").exists());
}

#[test]
fn restore_skip_existing_keeps_file_blocking_backup_directory() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "source/folder/child", "saved");
    write(
        root,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [folder]\n    to: files\n",
    );
    assert!(
        cli(root, &["backup", "config.yaml", "-a"], "")
            .status
            .success()
    );
    fs::remove_dir_all(root.join("source/folder")).unwrap();
    write(root, "source/folder", "local file");
    let skipped = cli(root, &["restore", "config.yaml", "--skip-existing"], "");
    assert!(skipped.status.success(), "{}", text(&skipped));
    assert_eq!(
        fs::read_to_string(root.join("source/folder")).unwrap(),
        "local file"
    );
    let overridden = cli(root, &["restore", "config.yaml", "--override"], "");
    assert!(overridden.status.success(), "{}", text(&overridden));
    assert_eq!(
        fs::read_to_string(root.join("source/folder/child")).unwrap(),
        "saved"
    );
}

#[test]
fn restore_reviews_each_conflicting_file_and_preserves_local_only_files() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "source/folder/one", "saved one\n");
    write(root, "source/folder/two", "saved two\n");
    write(
        root,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [folder]\n    to: files\n",
    );
    assert!(
        cli(root, &["backup", "config.yaml", "-a"], "")
            .status
            .success()
    );
    write(root, "source/folder/one", "local one\n");
    write(root, "source/folder/two", "local two\n");
    write(root, "source/folder/extra", "keep me\n");

    let declined = cli(root, &["restore", "config.yaml", "-a"], "n\nn\n");
    assert!(declined.status.success(), "{}", text(&declined));
    let output = text(&declined);
    assert_eq!(
        output.matches("Overwrite local file?").count(),
        2,
        "{output}"
    );
    assert!(
        output.contains("-saved one") && output.contains("+local one"),
        "{output}"
    );
    assert_eq!(
        fs::read_to_string(root.join("source/folder/one")).unwrap(),
        "local one\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("source/folder/two")).unwrap(),
        "local two\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("source/folder/extra")).unwrap(),
        "keep me\n"
    );

    let accepted = cli(root, &["restore", "config.yaml", "-a"], "y\ny\n");
    assert!(accepted.status.success(), "{}", text(&accepted));
    assert_eq!(
        fs::read_to_string(root.join("source/folder/one")).unwrap(),
        "saved one\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("source/folder/two")).unwrap(),
        "saved two\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("source/folder/extra")).unwrap(),
        "keep me\n"
    );
}

#[test]
fn restore_conflict_requires_an_answer_even_with_approved_hooks() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "source/file", "saved\n");
    write(
        root,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [file]\n    to: files\n",
    );
    assert!(
        cli(root, &["backup", "config.yaml", "-a"], "")
            .status
            .success()
    );
    write(root, "source/file", "local\n");
    let result = cli(root, &["restore", "config.yaml", "-a"], "");
    assert_eq!(result.status.code(), Some(1), "{}", text(&result));
    assert!(text(&result).contains("Overwrite local file?"));
    assert_eq!(
        fs::read_to_string(root.join("source/file")).unwrap(),
        "local\n"
    );
}

#[test]
fn dry_run_clean_backup_lists_changes_without_writing_or_running_hooks() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/file", "new");
    write(p, "backup/stale", "old");
    write(
        p,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nclean: true\nrules:\n  - from: source\n    targets: [file]\n    to: files\nhooks:\n  pre-backup:\n    - name: marker\n      command: [sh, -c, 'touch hook-ran']\n",
    );
    let out = cli(p, &["backup", "config.yaml", "--dry-run"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("Would remove:"), "{}", text(&out));
    assert!(text(&out).contains("backup/stale"), "{}", text(&out));
    assert!(text(&out).contains("Would copy"), "{}", text(&out));
    assert!(text(&out).contains("source/file"), "{}", text(&out));
    assert_eq!(fs::read_to_string(p.join("backup/stale")).unwrap(), "old");
    assert!(!p.join("backup/files/file").exists());
    assert!(!p.join("backup/.cockup-incomplete").exists());
    assert!(!p.join("hook-ran").exists());
    assert!(!text(&out).contains("Continue?"));
}

#[test]
fn dry_run_restore_lists_changes_without_writing_or_running_hooks() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/file", "source");
    write(
        p,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [file]\n    to: files\nhooks:\n  pre-restore:\n    - name: marker\n      command: [sh, -c, 'touch hook-ran']\n",
    );
    let backup = cli(p, &["backup", "config.yaml", "-a"], "");
    assert!(backup.status.success(), "{}", text(&backup));
    write(p, "source/file", "changed");
    let out = cli(p, &["restore", "config.yaml", "--dry-run"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("Would replace"), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(p.join("source/file")).unwrap(),
        "changed"
    );
    assert!(!p.join("hook-ran").exists());
    assert!(!text(&out).contains("Continue?"));
}

#[test]
fn dry_run_backup_rejects_non_directory_destination() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "backup", "not a directory");
    write(p, "source/file", "contents");
    write(
        p,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [file]\n    to: files\n",
    );
    let out = cli(p, &["backup", "config.yaml", "--dry-run"], "");
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(p.join("backup")).unwrap(),
        "not a directory"
    );
}

#[test]
fn backup_restore_preserves_nested_include_paths_and_glob_layout() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "configs/child/source/app/settings.txt", "included");
    write(p, "configs/source/top.txt", "top");
    write(
        p,
        "configs/child/deep/grand.yaml",
        "symlinks: reference\ndestination: ignored\nrules: []\n",
    );
    write(
        p,
        "configs/child/child.yaml",
        "symlinks: reference\ndestination: ignored\ninclude: [{file: deep/grand.yaml}]\nrules:\n  - from: source/*\n    targets: ['*.txt']\n    to: child\n",
    );
    write(
        p,
        "configs/root.yaml",
        "symlinks: reference\ndestination: ../backup\ninclude: [{file: child/child.yaml}]\nrules:\n  - from: source\n    targets: [top.txt]\n    to: top\n",
    );
    let out = cli(p, &["backup", "configs/root.yaml", "-a"], "");
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
    let out = cli(p, &["restore", "configs/root.yaml", "-a"], "");
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
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [folder, broken]\n    to: files\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-a"], "");
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
        cli(p, &["backup", "config.yaml", "-a"], "")
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
        "symlinks: reference\ndestination: source\nclean: true\nrules:\n  - from: source\n    targets: [file]\n    to: .\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-a"], "");
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
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [sub/file, good]\n    to: files\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-a"], "");
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
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [file]\n    to: files\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-a"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::metadata(p.join("backup/files/file"))
            .unwrap()
            .st_flags()
            & 1,
        1
    );
}
