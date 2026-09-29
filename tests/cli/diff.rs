use super::*;

#[test]
fn diff_reports_changed_missing_and_extra_files_without_writes_or_hooks() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "source/files/changed", "original");
    write(root, "source/files/missing", "restore me");
    write(
        root,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [files]\n    to: files\nhooks:\n  pre-restore:\n    - name: should-not-run\n      command: [sh, -c, 'touch hook-ran']\n",
    );
    assert!(
        cli(root, &["backup", "config.yaml", "-q"], "")
            .status
            .success()
    );

    let same = cli(root, &["diff", "config.yaml"], "");
    assert!(same.status.success(), "{}", text(&same));
    assert!(text(&same).contains("No differences"), "{}", text(&same));

    write(root, "source/files/changed", "new contents");
    fs::remove_file(root.join("source/files/missing")).unwrap();
    write(root, "source/files/extra", "local only");
    let result = cli(root, &["diff", "config.yaml"], "");
    assert!(result.status.success(), "{}", text(&result));
    let output = text(&result);
    for expected in ["changed", "missing", "extra", "3 differences"] {
        assert!(output.contains(expected), "{output}");
    }
    assert!(!root.join("hook-ran").exists());
    assert_eq!(
        fs::read_to_string(root.join("source/files/changed")).unwrap(),
        "new contents"
    );
    assert!(!root.join("source/files/missing").exists());
}

#[test]
fn diff_respects_reference_and_dereference_symlink_modes() {
    use std::os::unix::fs::symlink;
    for (mode, expected) in [("reference", "No differences"), ("dereference", "Changed")] {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write(root, "original", "saved");
        fs::create_dir(root.join("source")).unwrap();
        symlink("../original", root.join("source/link")).unwrap();
        write(
            root,
            "config.yaml",
            &format!(
                "symlinks: {mode}\ndestination: backup\nrules:\n  - from: source\n    targets: [link]\n    to: files\n"
            ),
        );
        assert!(
            cli(root, &["backup", "config.yaml", "-q"], "")
                .status
                .success()
        );
        write(root, "original", "modified");
        let result = cli(root, &["diff", "config.yaml"], "");
        assert!(result.status.success(), "{}", text(&result));
        assert!(text(&result).contains(expected), "{}", text(&result));
        assert_eq!(
            fs::read_link(root.join("source/link")).unwrap(),
            Path::new("../original")
        );
    }
}

#[test]
fn diff_rejects_incomplete_backup() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "source/file", "saved");
    write(
        root,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [file]\n    to: files\n",
    );
    assert!(
        cli(root, &["backup", "config.yaml", "-q"], "")
            .status
            .success()
    );
    write(root, "backup/.cockup-incomplete", "");
    let result = cli(root, &["diff", "config.yaml"], "");
    assert_eq!(result.status.code(), Some(1), "{}", text(&result));
    assert!(text(&result).contains("incomplete"));
}

#[test]
fn diff_rejects_backup_link_with_wrong_type() {
    use std::os::unix::fs::symlink;
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "original", "saved");
    fs::create_dir(root.join("source")).unwrap();
    symlink("../original", root.join("source/link")).unwrap();
    write(
        root,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [link]\n    to: files\n",
    );
    assert!(
        cli(root, &["backup", "config.yaml", "-q"], "")
            .status
            .success()
    );
    fs::remove_file(root.join("backup/files/link")).unwrap();
    write(root, "backup/files/link", "wrong type");
    let result = cli(root, &["diff", "config.yaml"], "");
    assert_eq!(result.status.code(), Some(1), "{}", text(&result));
    assert!(text(&result).contains("wrong type"), "{}", text(&result));
}

#[test]
fn diff_asks_once_for_cross_user_home_mapping() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let alice = root.join("alice");
    let bob = root.join("bob");
    write(&alice, "config/file", "saved");
    write(&bob, "config/file", "different");
    write(
        root,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules:\n  - from: '~/config'\n    targets: [file]\n    to: files\n",
    );
    let backup = Command::new(env!("CARGO_BIN_EXE_cockup"))
        .current_dir(root)
        .env("HOME", &alice)
        .env("USER", "alice")
        .args(["backup", "config.yaml", "-q"])
        .output()
        .unwrap();
    assert!(backup.status.success(), "{}", text(&backup));

    for (answer, expected) in [("c\n", "Changed"), ("o\n", "No differences")] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_cockup"))
            .current_dir(root)
            .env("HOME", &bob)
            .env("USER", "bob")
            .args(["diff", "config.yaml"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(answer.as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(result.status.success(), "{}", text(&result));
        assert_eq!(text(&result).matches("Compare user paths").count(), 1);
        assert!(text(&result).contains(expected), "{}", text(&result));
    }
    assert_eq!(
        fs::read_to_string(alice.join("config/file")).unwrap(),
        "saved"
    );
    assert_eq!(
        fs::read_to_string(bob.join("config/file")).unwrap(),
        "different"
    );
}
