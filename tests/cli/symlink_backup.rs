use super::*;

#[test]
fn preserve_symlinks_to_ancestors_and_reject_copying_onto_itself() {
    use std::os::unix::fs::symlink;
    let d = TempDir::new().unwrap();
    let p = d.path();
    fs::create_dir(p.join("source")).unwrap();
    symlink("/", p.join("source/root")).unwrap();
    symlink(p, p.join("source/ancestor")).unwrap();
    symlink("missing", p.join("source/broken")).unwrap();
    write(
        p,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    targets: [root, ancestor, broken]\n    to: files\n",
    );
    for command in ["backup", "backup", "restore"] {
        let out = cli(p, &[command, "config.yaml", "-a"], "");
        assert!(out.status.success(), "{}", text(&out));
        assert_eq!(
            fs::read_link(p.join("backup/files/root")).unwrap(),
            Path::new("/")
        );
        assert_eq!(fs::read_link(p.join("source/ancestor")).unwrap(), p);
        assert_eq!(
            fs::read_link(p.join("backup/files/broken")).unwrap(),
            Path::new("../../source/missing")
        );
    }
    write(
        p,
        "config.yaml",
        "symlinks: reference\ndestination: source\nrules:\n  - from: source\n    targets: [root]\n    to: .\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-a"], "");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        fs::read_link(p.join("source/root")).unwrap(),
        Path::new("/")
    );
}

#[test]
fn dereference_copies_file_and_directory_contents_and_continues_after_bad_links() {
    use std::os::unix::fs::symlink;
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "original/folder/file", "contents");
    fs::create_dir(p.join("source")).unwrap();
    symlink("../original/folder/file", p.join("source/file")).unwrap();
    symlink("../original/folder", p.join("source/folder")).unwrap();
    symlink("missing", p.join("source/broken")).unwrap();
    symlink(".", p.join("original/folder/cycle")).unwrap();
    write(
        p,
        "config.yaml",
        "symlinks: dereference\ndestination: backup\nrules:\n  - from: source\n    targets: [broken, folder, file]\n    to: files\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-a"], "");
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    for file in ["backup/files/file", "backup/files/folder/file"] {
        assert_eq!(fs::read_to_string(p.join(file)).unwrap(), "contents");
        assert!(!fs::symlink_metadata(p.join(file)).unwrap().is_symlink());
    }
    assert!(
        !fs::symlink_metadata(p.join("backup/files/folder"))
            .unwrap()
            .is_symlink()
    );
    assert!(text(&out).contains("cycle"));
    fs::remove_file(p.join("original/folder/cycle")).unwrap();
    fs::remove_file(p.join("source/broken")).unwrap();
    assert!(
        cli(p, &["backup", "config.yaml", "-a"], "")
            .status
            .success()
    );
    let out = cli(p, &["restore", "config.yaml", "-a"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        fs::symlink_metadata(p.join("source/file"))
            .unwrap()
            .is_symlink()
    );
    assert_eq!(
        fs::read_to_string(p.join("original/folder/file")).unwrap(),
        "contents"
    );
}

#[test]
fn prompt_chooses_once_for_all_links_and_requires_an_explicit_answer() {
    use std::os::unix::fs::symlink;
    for answer in ["r\n", "d\n"] {
        let d = TempDir::new().unwrap();
        let p = d.path();
        write(p, "original", "contents");
        fs::create_dir(p.join("source")).unwrap();
        for name in ["one", "two"] {
            symlink("../original", p.join("source").join(name)).unwrap();
        }
        write(
            p,
            "config.yaml",
            "symlinks: prompt\ndestination: backup\nrules:\n  - from: source\n    targets: [one, two]\n    to: files\n",
        );
        for invalid in ["", "invalid\n"] {
            let out = cli(p, &["backup", "config.yaml", "-a"], invalid);
            assert_eq!(out.status.code(), Some(1));
            assert!(!p.join("backup").exists());
        }
        let retried = cli(
            p,
            &["backup", "config.yaml", "-a"],
            &format!("invalid\n{answer}"),
        );
        assert!(retried.status.success(), "{}", text(&retried));
        assert_eq!(text(&retried).matches("[r]eference").count(), 2);
        let out = cli(p, &["backup", "config.yaml", "-a"], answer);
        assert!(out.status.success(), "{}", text(&out));
        assert_eq!(text(&out).matches("[r]eference").count(), 1);
        assert!(
            String::from_utf8_lossy(&out.stdout)
                .contains("[r]eference link or [d]ereference target?")
        );
        for name in ["one", "two"] {
            let dest = p.join("backup/files").join(name);
            assert_eq!(
                fs::symlink_metadata(&dest).unwrap().is_symlink(),
                answer == "r\n"
            );
            assert_eq!(fs::read_to_string(dest).unwrap(), "contents");
        }
    }
}

#[test]
fn root_symlink_policy_controls_included_rules_and_dereference_rejects_overlap() {
    use std::os::unix::fs::symlink;
    let d = TempDir::new().unwrap();
    let p = d.path();
    fs::create_dir(p.join("source")).unwrap();
    symlink("/", p.join("source/root")).unwrap();
    write(
        p,
        "child.yaml",
        "symlinks: dereference\ndestination: ignored\nrules:\n  - from: source\n    targets: [root]\n    to: files\n",
    );
    write(
        p,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules: []\ninclude: [{file: child.yaml}]\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-a"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_link(p.join("backup/files/root")).unwrap(),
        Path::new("/")
    );
    write(
        p,
        "config.yaml",
        "symlinks: dereference\ndestination: backup\nrules: []\ninclude: [{file: child.yaml}]\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-a"], "");
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains("Overlapping"));
    assert_eq!(
        fs::read_link(p.join("backup/files/root")).unwrap(),
        Path::new("/")
    );
}

#[test]
fn reference_backup_retains_targets_and_cleanup_does_not_follow_links() {
    use std::os::unix::fs::symlink;
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/original/file", "untouched");
    symlink("original/file", p.join("source/link")).unwrap();
    symlink("original", p.join("source/directory-link")).unwrap();
    write(
        p,
        "config.yaml",
        "symlinks: reference\nclean: true\ndestination: backup\nrules:\n  - from: source\n    targets: [link, directory-link]\n    to: files\n",
    );
    for _ in 0..2 {
        let out = cli(p, &["backup", "config.yaml", "-a"], "");
        assert!(out.status.success(), "{}", text(&out));
        assert_eq!(
            fs::read_to_string(p.join("backup/files/link")).unwrap(),
            "untouched"
        );
        assert_eq!(
            fs::canonicalize(p.join("backup/files/directory-link")).unwrap(),
            fs::canonicalize(p.join("source/original")).unwrap()
        );
        assert_eq!(
            fs::read_to_string(p.join("source/original/file")).unwrap(),
            "untouched"
        );
    }
    fs::remove_file(p.join("source/link")).unwrap();
    let out = cli(p, &["restore", "config.yaml", "-a"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_link(p.join("source/link")).unwrap(),
        Path::new("original/file")
    );
}

#[test]
fn nested_prompt_choices_are_collected_before_replacing_directories() {
    use std::os::unix::fs::symlink;
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "original/file", "saved");
    fs::create_dir_all(p.join("source/folder")).unwrap();
    symlink("../../original/file", p.join("source/folder/link")).unwrap();
    write(p, "backup/files/folder/keep", "old backup");
    write(
        p,
        "config.yaml",
        "symlinks: prompt\ndestination: backup\nrules:\n  - from: source\n    targets: [folder]\n    to: files\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-a"], "");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        fs::read_to_string(p.join("backup/files/folder/keep")).unwrap(),
        "old backup"
    );
    let out = cli(p, &["backup", "config.yaml", "-a"], "d\n");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(text(&out).matches("Symlink ").count(), 1);
    fs::remove_dir_all(p.join("source")).unwrap();
    fs::remove_file(p.join("original/file")).unwrap();
    let out = cli(p, &["restore", "config.yaml", "-a"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert!(!text(&out).contains("[r]eference link or [d]ereference target?"));
    assert_eq!(
        fs::read_link(p.join("source/folder/link")).unwrap(),
        Path::new("../../original/file")
    );
    assert_eq!(
        fs::read_to_string(p.join("original/file")).unwrap(),
        "saved"
    );
}

#[test]
fn source_links_may_point_into_backup_destination() {
    use std::os::unix::fs::symlink;
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "backup/original", "original content");
    fs::create_dir(p.join("source")).unwrap();
    symlink("../backup/original", p.join("source/link")).unwrap();
    write(
        p,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nclean: false\nrules:\n  - from: source\n    targets: [link]\n    to: files\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-a"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(p.join("backup/files/link")).unwrap(),
        "original content"
    );
}

#[test]
fn symlink_prompt_is_yellow_with_choices_on_the_next_line() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/original", "contents");
    std::os::unix::fs::symlink("original", p.join("source/link")).unwrap();
    write(
        p,
        "config.yaml",
        "symlinks: prompt\ndestination: backup\nrules:\n  - from: source\n    targets: [link]\n    to: files\n",
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_cockup"))
        .current_dir(p)
        .args(["backup", "config.yaml", "-a"])
        .env_remove("NO_COLOR")
        .env("FORCE_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"r\n").unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("\x1b[36;1m=> \x1b[0m\x1b[33;1mSymlink "));
    assert!(stderr.contains(" -> original:\x1b[0m"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(
        "\x1b[37;1m[r]eference link or [d]ereference target? (applies to all symlinks): \x1b[0m"
    ));
}

#[test]
fn switching_reference_to_dereference_replaces_old_directory_links() {
    for clean in [true, false] {
        let d = TempDir::new().unwrap();
        let p = d.path();
        write(p, "source/original/dir/item", "unchanged");
        std::os::unix::fs::symlink("original/dir", p.join("source/link")).unwrap();
        write(
            p,
            "config.yaml",
            &format!(
                "symlinks: prompt\nclean: {clean}\ndestination: backup\nrules:\n  - from: source\n    targets: [link]\n    to: files\n"
            ),
        );
        for answer in ["r\n", "d\n"] {
            let out = cli(p, &["backup", "config.yaml", "-a"], answer);
            assert!(out.status.success(), "{}", text(&out));
        }
        assert!(
            !fs::symlink_metadata(p.join("backup/files/link"))
                .unwrap()
                .is_symlink()
        );
        assert_eq!(
            fs::read_to_string(p.join("backup/files/link/item")).unwrap(),
            "unchanged"
        );
        assert_eq!(
            fs::read_to_string(p.join("source/original/dir/item")).unwrap(),
            "unchanged"
        );
    }
}

#[test]
fn reference_links_preserve_parent_traversal_after_symlinks() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/file", "wrong");
    write(p, "external/file", "correct");
    fs::create_dir(p.join("external/nested")).unwrap();
    std::os::unix::fs::symlink("../external/nested", p.join("source/alias")).unwrap();
    for name in ["file", "missing"] {
        std::os::unix::fs::symlink(
            format!("alias/../{name}"),
            p.join("source").join(format!("{name}-link")),
        )
        .unwrap();
    }
    write(
        p,
        "config.yaml",
        "symlinks: reference\nclean: true\ndestination: backup\nrules:\n  - from: source\n    targets: [file-link, missing-link]\n    to: files\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-a"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(p.join("backup/files/file-link")).unwrap(),
        "correct"
    );
    write(p, "external/missing", "new target");
    assert_eq!(
        fs::read_to_string(p.join("backup/files/missing-link")).unwrap(),
        "new target"
    );
    fs::remove_file(p.join("source/file-link")).unwrap();
    let out = cli(p, &["restore", "config.yaml", "-a"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_link(p.join("source/file-link")).unwrap(),
        Path::new("alias/../file")
    );
}
