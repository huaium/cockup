use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

fn cli(dir: &Path, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cockup"))
        .current_dir(dir)
        .args(args)
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}
fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}
fn write(dir: &Path, path: &str, content: &str) {
    let path = dir.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}
#[test]
fn cli_exposes_commands_and_version() {
    let dir = TempDir::new().unwrap();
    let out = cli(dir.path(), &["--help"], "");
    assert!(out.status.success());
    for word in ["backup", "restore", "hook", "list"] {
        assert!(text(&out).contains(word), "{}", text(&out));
    }
    assert!(text(&cli(dir.path(), &["--version"], "")).contains("0.2.0"));
    assert_eq!(cli(dir.path(), &["unknown"], "").status.code(), Some(2));
}

#[test]
fn invalid_configuration_fails_before_side_effects() {
    let dir = TempDir::new().unwrap();
    for yaml in [
        "rules: []",
        "symlinks: referece\ndestination: backup\nrules: [bad]",
        "symlinks: referece\ndestination: backup\nrules: []\ninclude: [missing.yaml]",
        "symlinks: referece\ndestination: backup\nrules: []\ninclude: [config.yaml]",
    ] {
        write(dir.path(), "config.yaml", yaml);
        let out = cli(dir.path(), &["backup", "config.yaml", "-q"], "");
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(!dir.path().join("backup").exists());
        assert!(!out.stderr.is_empty());
    }
}

#[test]
fn symlink_policy_is_required_and_validated_before_side_effects() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    for policy in ["", "symlinks: unknown\n", "symlinks: null\n"] {
        write(p, "backup/keep", "old backup");
        write(
            p,
            "config.yaml",
            &format!("{policy}destination: backup\nclean: true\nrules: []\n"),
        );
        let out = cli(p, &["backup", "config.yaml", "-q"], "");
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(text(&out).contains("symlinks") || text(&out).contains("unknown"));
        assert_eq!(
            fs::read_to_string(p.join("backup/keep")).unwrap(),
            "old backup"
        );
    }
}

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
        "symlinks: referece\ndestination: backup\nrules:\n  - from: source\n    targets: [root, ancestor, broken]\n    to: files\n",
    );
    for command in ["backup", "backup", "restore"] {
        let out = cli(p, &[command, "config.yaml", "-q"], "");
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
        "symlinks: referece\ndestination: source\nrules:\n  - from: source\n    targets: [root]\n    to: .\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
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
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
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
        cli(p, &["backup", "config.yaml", "-q"], "")
            .status
            .success()
    );
    let out = cli(p, &["restore", "config.yaml", "-q"], "");
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
            let out = cli(p, &["backup", "config.yaml", "-q"], invalid);
            assert_eq!(out.status.code(), Some(1));
            assert!(!p.join("backup").exists());
        }
        let out = cli(p, &["backup", "config.yaml", "-q"], answer);
        assert!(out.status.success(), "{}", text(&out));
        assert_eq!(text(&out).matches("[r]eferece").count(), 1);
        assert!(text(&out).contains("\n[r]eferece link or [d]ereference target?"));
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
        "symlinks: referece\ndestination: backup\nrules: []\ninclude: [child.yaml]\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_link(p.join("backup/files/root")).unwrap(),
        Path::new("/")
    );
    write(
        p,
        "config.yaml",
        "symlinks: dereference\ndestination: backup\nrules: []\ninclude: [child.yaml]\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains("Overlapping"));
    assert_eq!(
        fs::read_link(p.join("backup/files/root")).unwrap(),
        Path::new("/")
    );
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
        "symlinks: referece\nclean: true\ndestination: backup\nrules:\n  - from: source\n    targets: [link, directory-link]\n    to: files\n",
    );
    for _ in 0..2 {
        let out = cli(p, &["backup", "config.yaml", "-q"], "");
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
    let out = cli(p, &["restore", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_link(p.join("source/link")).unwrap(),
        Path::new("original/file")
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
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        fs::read_to_string(p.join("backup/files/folder/keep")).unwrap(),
        "old backup"
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "d\n");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(text(&out).matches("Symlink ").count(), 1);
    fs::remove_dir_all(p.join("source")).unwrap();
    fs::remove_file(p.join("original/file")).unwrap();
    let out = cli(p, &["restore", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert!(!text(&out).contains("Symlink "));
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
    for answer in ["", "original\n"] {
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
        assert_eq!(text(&out).matches("Restore user paths").count(), 1);
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
fn included_hooks_prompt_once_and_quiet_suppresses_confirmation() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "nested/child.yaml",
        "symlinks: referece\ndestination: ignored\nrules: []\nhooks:\n  pre-backup:\n    - name: child\n      command: [echo, hello]\n",
    );
    write(
        dir.path(),
        "config.yaml",
        "symlinks: referece\ndestination: backup\nrules: []\ninclude: [nested/child.yaml]\n",
    );
    let denied = cli(dir.path(), &["backup", "config.yaml"], "n\n");
    assert!(denied.status.success());
    assert_eq!(text(&denied).matches("Continue?").count(), 1);
    assert!(!dir.path().join("backup").exists());
    let quiet = cli(dir.path(), &["backup", "config.yaml", "-q"], "");
    assert!(!text(&quiet).contains("Continue?"));
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
        "symlinks: referece\ndestination: ignored\nrules: []\n",
    );
    write(
        p,
        "configs/child/child.yaml",
        "symlinks: referece\ndestination: ignored\ninclude: [deep/grand.yaml]\nrules:\n  - from: source/*\n    targets: ['*.txt']\n    to: child\n",
    );
    write(
        p,
        "configs/root.yaml",
        "symlinks: referece\ndestination: ../backup\ninclude: [child/child.yaml]\nrules:\n  - from: source\n    targets: [top.txt]\n    to: top\n",
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
fn hooks_run_in_order_with_checks_environment_and_working_directories() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/file", "content");
    write(
        p,
        "config.yaml",
        r#"symlinks: referece
destination: backup
rules:
  - from: source
    to: files
    targets: [file]
    on-start:
      - name: start
        command: [sh, -c, 'printf start >> ../order; test ! -f files/file']
    on-end:
      - name: end
        command: [sh, -c, 'test -f files/file && printf end >> ../order']
hooks:
  pre-backup:
    - name: pre
      command: [sh, -c, 'printf "$TOKEN" > order']
      check: [sh, -c, 'test "$(cat order)" = pre']
      env: {TOKEN: pre, NUMBER: 12}
  post-backup:
    - name: post
      command: [sh, -c, 'printf post >> ../order']
      check: [sh, -c, 'test "$(cat ../order)" = prestartendpost']
"#,
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(p.join("order")).unwrap(),
        "prestartendpost"
    );
    assert!(text(&out).contains("Completed 1/1 hook"));
}

#[test]
fn named_and_interactive_hooks_report_failed_checks_and_continue() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(
        p,
        "config.yaml",
        r#"symlinks: referece
destination: backup
rules: []
hooks:
  pre-backup:
    - name: bad
      command: [sh, -c, 'printf main']
      check: [sh, -c, 'exit 1']
      output: true
    - name: good
      command: [sh, -c, 'printf good > marker']
"#,
    );
    let out = cli(p, &["hook", "config.yaml", "-q", "-n", "bad"], "");
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains("Completed 0/1 hook. Error: 1 hook failed."));
    assert!(text(&out).contains("check"));
    assert!(!p.join("marker").exists());
    let out = cli(p, &["hook", "config.yaml", "--yes"], "1,2\n");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(fs::read_to_string(p.join("marker")).unwrap(), "good");
    assert!(text(&out).contains("Completed 1/2 hooks"));
    assert_eq!(
        cli(p, &["hook", "config.yaml", "-y", "-n", "missing"], "")
            .status
            .code(),
        Some(1)
    );
    assert_eq!(
        cli(p, &["hook", "config.yaml", "-q"], "bogus\n")
            .status
            .code(),
        Some(1)
    );
}

#[test]
fn timed_out_commands_and_checks_fail_and_skip_dependent_work() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(
        p,
        "config.yaml",
        r#"symlinks: referece
destination: backup
rules: []
hooks:
  pre-backup:
    - name: command-timeout
      command: [sh, -c, 'echo $$ > pid; exec sleep 2']
      check: [sh, -c, 'touch should-not-exist']
      timeout: 1
    - name: check-timeout
      command: [true]
      check: [sleep, '2']
      timeout: 1
"#
        .replace("[true]", "['true']")
        .as_str(),
    );
    for name in ["command-timeout", "check-timeout"] {
        let out = cli(p, &["hook", "config.yaml", "-q", "-n", name], "");
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(text(&out).contains("timed out"));
    }
    assert!(!p.join("should-not-exist").exists());
    let pid = fs::read_to_string(p.join("pid")).unwrap();
    assert!(
        !Command::new("kill")
            .args(["-0", pid.trim()])
            .output()
            .unwrap()
            .status
            .success()
    );
}

fn executable(dir: &Path, name: &str, script: &str) {
    use std::os::unix::fs::PermissionsExt;
    write(dir, name, script);
    fs::set_permissions(dir.join(name), fs::Permissions::from_mode(0o755)).unwrap();
}
#[test]
fn brew_discovery_uses_json_and_reports_partial_failure_in_sorted_output() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    executable(
        p,
        "brew",
        r#"#!/bin/sh
[ "$HOMEBREW_NO_AUTO_UPDATE" = 1 ] || exit 4
case "$1" in
  --version) echo Homebrew ;;
  list) printf 'zeta\nalpha\nmissing\n' ;;
  info)
    case "$4" in
      missing) exit 2 ;;
      alpha) printf '%s\n' '{"casks":[{"artifacts":[{"zap":[{"trash":"~/alpha","rmdir":["~/empty"]}]}]}]}' ;;
      zeta) printf '%s\n' '{"casks":[{"artifacts":[{"zap":[{"trash":["~/zeta"]}]}]}]}' ;;
    esac ;;
esac
"#,
    );
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_cockup"))
            .current_dir(p)
            .args(args)
            .env("PATH", p)
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    };
    let out = run(&["list"]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    let output = String::from_utf8_lossy(&out.stdout);
    assert!(output.find("alpha:").unwrap() < output.find("zeta:").unwrap());
    assert!(output.contains("~/alpha") && output.contains("~/empty"));
    assert!(String::from_utf8_lossy(&out.stderr).contains("missing"));
    assert!(run(&["list", "zeta"]).status.success());
    fs::remove_file(p.join("brew")).unwrap();
    let out = run(&["list"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out).contains("Homebrew"));
}

#[test]
fn failures_are_red_and_backup_continues_after_copy_or_hook_errors() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/file", "kept");
    write(
        p,
        "config.yaml",
        r#"symlinks: referece
destination: backup
rules:
  - from: source
    targets: ['missing*', file]
    to: files
hooks:
  pre-backup:
    - name: failed-command
      command: [sh, -c, 'exit 1']
      check: [sh, -c, 'touch forbidden']
    - name: missing-command
      command: [cockup-test-nonexistent-command]
  post-backup:
    - name: finish
      command: [sh, -c, 'touch ../finished']
"#,
    );
    let out = Command::new(env!("CARGO_BIN_EXE_cockup"))
        .current_dir(p)
        .args(["backup", "config.yaml", "-q"])
        .env_remove("NO_COLOR")
        .env("FORCE_COLOR", "1")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains("\x1b[31;1m=> Completed 0/2 hooks. Error: 2 hooks failed.")
    );
    assert!(!p.join("forbidden").exists());
    assert!(p.join("finished").exists());
    assert_eq!(
        fs::read_to_string(p.join("backup/files/file")).unwrap(),
        "kept"
    );
    assert!(!text(&out).contains("Backup completed."));
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
fn restore_lifecycle_and_hook_output_settings_are_preserved() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "backup/files/file", "restored");
    write(
        p,
        "config.yaml",
        r#"symlinks: referece
destination: backup
rules:
  - from: source
    to: files
    targets: [file]
hooks:
  pre-restore:
    - name: before
      command: [sh, -c, 'test ! -e source/file; printf before > order; echo hidden']
  post-restore:
    - name: after
      command: [sh, -c, 'test -f ../source/file && printf after >> ../order; echo visible']
      check: [sh, -c, 'echo checked; test "$(cat ../order)" = beforeafter']
      output: true
"#,
    );
    let out = cli(p, &["restore", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert!(!text(&out).contains("hidden"));
    assert!(text(&out).contains("visible") && text(&out).contains("checked"));
    assert_eq!(
        fs::read_to_string(p.join("source/file")).unwrap(),
        "restored"
    );
}

#[test]
fn empty_null_and_invalid_hook_configuration_are_handled() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    for hook in [
        "name: invalid\n      command: []",
        "name: invalid\n      command: [echo]\n      check: []",
        "name: invalid\n      command: [echo]\n      timeout: -1",
    ] {
        write(
            p,
            "config.yaml",
            &format!(
                "symlinks: referece\ndestination: backup\nrules: []\nhooks:\n  pre-backup:\n    - {hook}\n"
            ),
        );
        assert_eq!(
            cli(p, &["backup", "config.yaml", "-q"], "").status.code(),
            Some(1)
        );
        assert!(!p.join("backup").exists());
    }
    write(
        p,
        "config.yaml",
        "symlinks: referece\ndestination: backup\nrules:\n  - from: source\n    to: .\n    targets: null\n    on-start: null\nhooks: null\n",
    );
    assert!(
        cli(p, &["backup", "config.yaml", "-q"], "")
            .status
            .success()
    );
    assert_eq!(
        cli(p, &["hook", "config.yaml", "-q"], "").status.code(),
        Some(1)
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

#[test]
fn home_expansion_and_dotfile_globs_work_without_shell_expansion() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "home/.config/app/.hidden", "hidden");
    write(p, "home/.config/app/file", "visible");
    write(
        p,
        "config.yaml",
        "symlinks: referece\ndestination: '~/backup'\nrules:\n  - from: '~/.config'\n    targets: ['app/*', 'app/.*']\n    to: .\n",
    );
    let out = Command::new(env!("CARGO_BIN_EXE_cockup"))
        .current_dir(p)
        .args(["backup", "config.yaml", "-q"])
        .env("HOME", p.join("home"))
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(p.join("home/backup/app/.hidden")).unwrap(),
        "hidden"
    );
    assert_eq!(
        fs::read_to_string(p.join("home/backup/app/file")).unwrap(),
        "visible"
    );
}

#[test]
fn no_arguments_shows_help_successfully() {
    let d = TempDir::new().unwrap();
    let out = cli(d.path(), &[], "");
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("backup"));
}

#[test]
fn documented_sample_checks_have_expected_exit_statuses() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "config.yaml", include_str!("../sample/hooks.yaml"));
    let pass = cli(
        p,
        &["hook", "config.yaml", "-q", "--name", "Check created file"],
        "",
    );
    assert!(pass.status.success(), "{}", text(&pass));
    assert!(text(&pass).contains("Check passed"));
    assert!(!p.join(".hook-check-result.txt").exists());
    let fail = cli(
        p,
        &["hook", "config.yaml", "-q", "--name", "Check failure"],
        "",
    );
    assert_eq!(fail.status.code(), Some(1));
    assert!(text(&fail).contains("Completed 0/1 hook. Error: 1 hook failed."));
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

#[test]
fn invalid_rule_paths_fail_before_cleaning_backup() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "backup/keep", "untouched");
    for fields in [
        "targets: [/absolute]\n    to: files",
        "targets: [../outside]\n    to: files",
        "targets: [file]\n    to: ../outside",
    ] {
        write(
            p,
            "config.yaml",
            &format!(
                "symlinks: referece\ndestination: backup\nclean: true\nrules:\n  - from: source\n    {fields}\n"
            ),
        );
        let out = cli(p, &["backup", "config.yaml", "-q"], "");
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert_eq!(
            fs::read_to_string(p.join("backup/keep")).unwrap(),
            "untouched"
        );
    }
}

#[test]
fn malformed_globs_fail_before_confirmation_hooks_or_cleanup() {
    for (from, target, pattern) in [("source", "[", "["), ("source/[", "file", "source/[")] {
        for included in [false, true] {
            for quiet in [false, true] {
                let d = TempDir::new().unwrap();
                let p = d.path();
                write(p, "backup/keep", "previous backup");
                let rules = format!(
                    "symlinks: referece\ndestination: backup\nrules:\n  - from: '{from}'\n    targets: ['{target}']\n    to: files\n"
                );
                let (config, invalid_file) = if included {
                    write(p, "nested/child.yaml", &rules);
                    (
                        "symlinks: referece\ndestination: backup\nrules: []\ninclude: [nested/child.yaml]\n"
                            .to_string(),
                        "child.yaml",
                    )
                } else {
                    (rules, "config.yaml")
                };
                write(
                    p,
                    "config.yaml",
                    &(config
                        + "clean: true\nhooks:\n  pre-backup:\n    - name: marker\n      command: [sh, -c, 'touch hook-ran']\n"),
                );
                let mut args = vec!["backup", "config.yaml"];
                if quiet {
                    args.push("-q");
                }
                let out = cli(p, &args, "y\n");
                assert_eq!(out.status.code(), Some(1), "{}", text(&out));
                assert_eq!(
                    fs::read_to_string(p.join("backup/keep")).unwrap(),
                    "previous backup"
                );
                assert!(!p.join("hook-ran").exists());
                assert!(!text(&out).contains("Continue?"));
                let error = String::from_utf8_lossy(&out.stderr);
                assert!(error.contains(invalid_file), "{error}");
                assert!(error.contains("rule 1"), "{error}");
                assert!(error.contains(pattern), "{error}");
            }
        }
    }
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
        "symlinks: referece\ndestination: backup\nclean: false\nrules:\n  - from: source\n    targets: [link]\n    to: files\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
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
        .args(["backup", "config.yaml", "-q"])
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
    assert!(stderr.contains("\x1b[33;1m=> Symlink "));
    assert!(stderr.contains(
        " -> original:\n[r]eferece link or [d]ereference target? (applies to all symlinks)\x1b[0m"
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
            let out = cli(p, &["backup", "config.yaml", "-q"], answer);
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
        "symlinks: referece\nclean: true\ndestination: backup\nrules:\n  - from: source\n    targets: [file-link, missing-link]\n    to: files\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
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
    let out = cli(p, &["restore", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_link(p.join("source/file-link")).unwrap(),
        Path::new("alias/../file")
    );
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
