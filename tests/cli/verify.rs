use super::*;

#[test]
fn read_only_backup_checks_warn_when_backup_is_not_initialized() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(
        root,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules: []\n",
    );
    for command in ["verify", "diff"] {
        let missing = cli(root, &[command, "config.yaml"], "");
        assert!(missing.status.success(), "{}", text(&missing));
        assert!(
            text(&missing).contains("Backup is not initialized"),
            "{}",
            text(&missing)
        );
        assert!(!root.join("backup").exists());

        write(root, "backup", "not a directory");
        let invalid = cli(root, &[command, "config.yaml"], "");
        assert_eq!(invalid.status.code(), Some(1), "{}", text(&invalid));
        assert!(
            text(&invalid).contains("not a directory"),
            "{}",
            text(&invalid)
        );
        fs::remove_file(root.join("backup")).unwrap();
    }
    write(
        root,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules: []\ninclude:\n  - ingredient: ghostty\n",
    );
    for command in ["verify", "diff"] {
        let missing = cli(root, &[command, "config.yaml"], "");
        assert!(missing.status.success(), "{}", text(&missing));
        assert!(text(&missing).contains("Backup is not initialized"));
    }
}

#[test]
fn verify_accepts_complete_backup_without_running_hooks() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/file", "saved");
    write(
        p,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [file]\n    to: files\nhooks:\n  pre-restore:\n    - name: marker\n      command: [sh, -c, 'touch hook-ran']\n",
    );
    let backup = cli(p, &["backup", "config.yaml", "-q"], "");
    assert!(backup.status.success(), "{}", text(&backup));
    let out = cli(p, &["verify", "config.yaml"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("Verification completed"));
    assert!(!p.join("hook-ran").exists());
    assert!(!text(&out).contains("Continue?"));
}

#[test]
fn verify_reports_missing_data_manifest_and_incomplete_backup() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/file", "saved");
    write(
        p,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [file]\n    to: files\n",
    );
    let backup = cli(p, &["backup", "config.yaml", "-q"], "");
    assert!(backup.status.success(), "{}", text(&backup));
    fs::remove_file(p.join("backup/files/file")).unwrap();
    let missing = cli(p, &["verify", "config.yaml"], "");
    assert_eq!(missing.status.code(), Some(1), "{}", text(&missing));
    assert!(text(&missing).contains("backup/files/file"));
    write(p, "backup/files/file", "saved");
    write(p, "backup/.cockup-incomplete", "");
    let incomplete = cli(p, &["verify", "config.yaml"], "");
    assert_eq!(incomplete.status.code(), Some(1));
    fs::remove_file(p.join("backup/.cockup-incomplete")).unwrap();
    fs::remove_file(p.join("backup/.cockup-symlinks.json")).unwrap();
    let no_manifest = cli(p, &["verify", "config.yaml"], "");
    assert_eq!(no_manifest.status.code(), Some(1));
    write(p, "backup/.cockup-symlinks.json", "not json");
    let invalid_manifest = cli(p, &["verify", "config.yaml"], "");
    assert_eq!(invalid_manifest.status.code(), Some(1));
}

#[test]
fn verify_checks_recorded_links_but_accepts_dangling_references() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    fs::create_dir(p.join("source")).unwrap();
    std::os::unix::fs::symlink("missing", p.join("source/link")).unwrap();
    write(
        p,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [link]\n    to: files\n",
    );
    let backup = cli(p, &["backup", "config.yaml", "-q"], "");
    assert!(backup.status.success(), "{}", text(&backup));
    let valid = cli(p, &["verify", "config.yaml"], "");
    assert!(valid.status.success(), "{}", text(&valid));
    fs::remove_file(p.join("backup/files/link")).unwrap();
    write(p, "backup/files/link", "wrong kind");
    let wrong_kind = cli(p, &["verify", "config.yaml"], "");
    assert_eq!(wrong_kind.status.code(), Some(1));
    assert!(text(&wrong_kind).contains("backup/files/link"));
}
