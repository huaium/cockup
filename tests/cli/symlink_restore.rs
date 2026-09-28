use super::*;

#[test]
fn dry_run_dereference_restore_previews_target_and_link_without_changes() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/original", "saved");
    std::os::unix::fs::symlink("original", p.join("source/link")).unwrap();
    write(
        p,
        "config.yaml",
        "symlinks: dereference\ndestination: backup\nrules:\n  - from: source\n    targets: [link]\n    to: files\n",
    );
    let backup = cli(p, &["backup", "config.yaml", "-q"], "");
    assert!(backup.status.success(), "{}", text(&backup));
    fs::remove_file(p.join("source/link")).unwrap();
    write(p, "source/original", "changed");
    let out = cli(p, &["restore", "config.yaml", "--dry-run"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("Would replace file:"), "{}", text(&out));
    assert!(
        text(&out).contains("Would restore symlink:"),
        "{}",
        text(&out)
    );
    assert_eq!(
        fs::read_to_string(p.join("source/original")).unwrap(),
        "changed"
    );
    assert!(!p.join("source/link").exists());
}

#[test]
fn manifest_restores_dereferenced_contents_and_recreates_original_links() {
    use std::os::unix::fs::symlink;
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "original/file", "saved content");
    fs::create_dir(p.join("source")).unwrap();
    symlink("../original/file", p.join("source/link")).unwrap();
    write(
        p,
        "config.yaml",
        "symlinks: dereference\ndestination: backup\nrules:\n  - from: source\n    targets: [link]\n    to: files\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(p.join("backup/.cockup-symlinks.json")).unwrap()).unwrap();
    assert_eq!(manifest["version"], 1);
    assert_eq!(manifest["links"][0]["target"], "../original/file");
    assert_eq!(manifest["links"][0]["mode"], "dereference");
    fs::remove_file(p.join("source/link")).unwrap();
    write(p, "original/file", "changed");
    let out = cli(p, &["restore", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_link(p.join("source/link")).unwrap(),
        Path::new("../original/file")
    );
    assert_eq!(
        fs::read_to_string(p.join("original/file")).unwrap(),
        "saved content"
    );
}

#[test]
fn restore_asks_once_to_map_backup_home_for_contents_and_links() {
    use std::os::unix::fs::symlink;
    for choice in ["current", "original"] {
        let d = TempDir::new().unwrap();
        let p = d.path();
        let old = p.join("users/alice");
        let new = p.join("users/bob");
        write(&old, "data/one", "one");
        write(&old, "data/two", "two");
        write(&old, "config/plain", "ordinary");
        symlink(old.join("data/one"), old.join("config/link1")).unwrap();
        symlink("../../alice/data/two", old.join("config/link2")).unwrap();
        write(
            p,
            "config.yaml",
            &format!(
                "symlinks: dereference\ndestination: backup\nrules:\n  - from: '{}'\n    targets: [plain, link1, link2]\n    to: files\n",
                old.join("config").display()
            ),
        );
        let out = Command::new(env!("CARGO_BIN_EXE_cockup"))
            .current_dir(p)
            .args(["backup", "config.yaml", "-q"])
            .env("HOME", &old)
            .env("USER", "alice")
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", text(&out));
        fs::remove_dir_all(&old).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_cockup"))
            .current_dir(p)
            .args(["restore", "config.yaml", "-q"])
            .env("HOME", &new)
            .env("USER", "bob")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        writeln!(child.stdin.take().unwrap(), "{choice}").unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success(), "{}", text(&out));
        assert_eq!(text(&out).matches("Restore user paths").count(), 1);
        let home = if choice == "current" { &new } else { &old };
        assert_eq!(
            fs::read_to_string(home.join("config/plain")).unwrap(),
            "ordinary"
        );
        assert_eq!(fs::read_to_string(home.join("data/one")).unwrap(), "one");
        assert_eq!(fs::read_to_string(home.join("data/two")).unwrap(), "two");
        assert_eq!(
            fs::read_link(home.join("config/link1")).unwrap(),
            home.join("data/one")
        );
        assert_eq!(
            fs::read_link(home.join("config/link2")).unwrap(),
            Path::new(if choice == "current" {
                "../data/two"
            } else {
                "../../alice/data/two"
            })
        );
        assert!(!if choice == "current" { &old } else { &new }.exists());
    }
}

#[test]
fn failed_backup_keeps_manifest_and_blocks_restore_until_a_successful_backup() {
    use std::os::unix::fs::symlink;
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "original", "saved");
    fs::create_dir(p.join("source")).unwrap();
    symlink("../original", p.join("source/link")).unwrap();
    write(
        p,
        "config.yaml",
        "symlinks: dereference\ndestination: backup\nrules:\n  - from: source\n    targets: [link]\n    to: files\n",
    );
    assert!(
        cli(p, &["backup", "config.yaml", "-q"], "")
            .status
            .success()
    );
    let previous = fs::read(p.join("backup/.cockup-symlinks.json")).unwrap();
    fs::remove_file(p.join("original")).unwrap();
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        fs::read(p.join("backup/.cockup-symlinks.json")).unwrap(),
        previous
    );
    let out = cli(p, &["restore", "config.yaml", "-q"], "");
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(!p.join("original").exists());
    write(p, "original", "new");
    assert!(
        cli(p, &["backup", "config.yaml", "-q"], "")
            .status
            .success()
    );
    fs::remove_file(p.join("original")).unwrap();
    assert!(
        cli(p, &["restore", "config.yaml", "-q"], "")
            .status
            .success()
    );
    assert_eq!(fs::read_to_string(p.join("original")).unwrap(), "new");
}

#[test]
fn restore_rejects_missing_or_invalid_manifest_before_writing_dereferenced_files() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "backup/files/link", "backup contents");
    write(p, "source/link", "untouched");
    write(
        p,
        "config.yaml",
        "symlinks: dereference\ndestination: backup\nrules:\n  - from: source\n    targets: [link]\n    to: files\n",
    );
    for manifest in [None, Some("not json"), Some("{\"version\":999}")] {
        if let Some(manifest) = manifest {
            write(p, "backup/.cockup-symlinks.json", manifest);
        }
        let out = cli(p, &["restore", "config.yaml", "-q"], "");
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert_eq!(
            fs::read_to_string(p.join("source/link")).unwrap(),
            "untouched"
        );
    }
}

#[test]
fn dereference_restores_a_chain_of_relative_symlinks() {
    use std::os::unix::fs::symlink;
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "original/file", "saved");
    fs::create_dir(p.join("source")).unwrap();
    symlink("../original/alias", p.join("source/link")).unwrap();
    symlink("file", p.join("original/alias")).unwrap();
    write(
        p,
        "config.yaml",
        "symlinks: dereference\ndestination: backup\nrules:\n  - from: source\n    targets: [link]\n    to: files\n",
    );
    assert!(
        cli(p, &["backup", "config.yaml", "-q"], "")
            .status
            .success()
    );
    fs::remove_file(p.join("source/link")).unwrap();
    fs::remove_dir_all(p.join("original")).unwrap();
    let out = cli(p, &["restore", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(fs::read_to_string(p.join("source/link")).unwrap(), "saved");
    assert_eq!(
        fs::read_link(p.join("source/link")).unwrap(),
        Path::new("../original/alias")
    );
    assert_eq!(
        fs::read_link(p.join("original/alias")).unwrap(),
        Path::new("file")
    );
}

#[test]
fn backup_reserves_manifest_paths_before_cleaning() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/.cockup-symlinks.json", "user file");
    write(p, "backup/keep", "keep");
    write(
        p,
        "config.yaml",
        "symlinks: referece\nclean: true\ndestination: backup\nrules:\n  - from: source\n    targets: ['.cockup-symlinks.json']\n    to: .\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert_eq!(fs::read_to_string(p.join("backup/keep")).unwrap(), "keep");
}

#[test]
fn home_choice_applies_to_tilde_rules_without_links_and_missing_input_aborts() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    let old = p.join("alice");
    let new = p.join("bob");
    write(&old, "config/file", "saved");
    write(
        p,
        "config.yaml",
        "symlinks: referece\ndestination: backup\nrules:\n  - from: '~/config'\n    targets: [file]\n    to: files\n",
    );
    let out = Command::new(env!("CARGO_BIN_EXE_cockup"))
        .current_dir(p)
        .args(["backup", "config.yaml", "-q"])
        .env("HOME", &old)
        .env("USER", "alice")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    fs::remove_dir_all(&old).unwrap();
    for answer in ["", "invalid\noriginal\n"] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_cockup"))
            .current_dir(p)
            .args(["restore", "config.yaml", "-q"])
            .env("HOME", &new)
            .env("USER", "bob")
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
        let out = child.wait_with_output().unwrap();
        assert_eq!(
            text(&out).matches("Restore user paths").count(),
            if answer.is_empty() { 1 } else { 2 }
        );
        assert!(!new.exists());
        if answer.is_empty() {
            assert_eq!(out.status.code(), Some(1));
            assert!(!old.exists());
        } else {
            assert!(out.status.success(), "{}", text(&out));
            assert_eq!(
                fs::read_to_string(old.join("config/file")).unwrap(),
                "saved"
            );
        }
    }
}

#[test]
fn clean_backup_replaces_previous_user_identity_and_link_records() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(
        p,
        "config.yaml",
        "symlinks: referece\nclean: true\ndestination: backup\nrules:\n  - from: '~/config'\n    targets: ['*']\n    to: files\n",
    );
    for user in ["alice", "bob"] {
        let home = p.join(user);
        write(&home, "config/file", user);
        if user == "alice" {
            std::os::unix::fs::symlink("file", home.join("config/link")).unwrap();
        }
        let out = Command::new(env!("CARGO_BIN_EXE_cockup"))
            .current_dir(p)
            .args(["backup", "config.yaml", "-q"])
            .env("HOME", &home)
            .env("USER", user)
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", text(&out));
    }
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(p.join("backup/.cockup-symlinks.json")).unwrap()).unwrap();
    assert_eq!(manifest["backup_user"], "bob");
    assert_eq!(manifest["backup_home"], p.join("bob").to_str().unwrap());
    assert_eq!(
        manifest["canonical_home"],
        fs::canonicalize(p.join("bob")).unwrap().to_str().unwrap()
    );
    assert!(manifest["links"].as_array().unwrap().is_empty());
    fs::remove_file(p.join("bob/config/file")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_cockup"))
        .current_dir(p)
        .args(["restore", "config.yaml", "-q"])
        .env("HOME", p.join("bob"))
        .env("USER", "bob")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    assert!(!text(&out).contains("Restore user paths"));
    assert_eq!(
        fs::read_to_string(p.join("bob/config/file")).unwrap(),
        "bob"
    );
    assert_eq!(
        fs::read_to_string(p.join("alice/config/file")).unwrap(),
        "alice"
    );
}
