use super::*;

#[test]
fn included_rules_are_wrapped_under_the_root_destination() {
    let dir = TempDir::new().unwrap();
    let p = dir.path();
    write(p, "source/file", "contents");
    write(
        p,
        "nested/child.yaml",
        "symlinks: referece\ndestination: ignored\nrules:\n  - from: ../source\n    targets: [file]\n    to: config\n",
    );
    write(
        p,
        "config.yaml",
        "symlinks: referece\ndestination: backup\ninclude:\n  - file: nested/child.yaml\n    wrap: imported\nrules: []\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(p.join("backup/imported/config/file")).unwrap(),
        "contents"
    );
    fs::remove_file(p.join("source/file")).unwrap();
    let out = cli(p, &["restore", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(p.join("source/file")).unwrap(),
        "contents"
    );
}

#[test]
fn nested_include_and_rule_settings_override_inherited_defaults() {
    use std::os::unix::fs::symlink;
    let dir = TempDir::new().unwrap();
    let p = dir.path();
    write(p, "source/original", "contents");
    for name in ["inherited", "nested", "local"] {
        symlink("original", p.join("source").join(name)).unwrap();
    }
    write(
        p,
        "nested/grand.yaml",
        "symlinks: prompt\ndestination: ignored\nrules:\n  - from: ../source\n    targets: [nested]\n    to: files\n    symlinks: referece\n  - from: ../source\n    targets: [local]\n    to: files\n    symlinks: dereference\n",
    );
    write(
        p,
        "nested/child.yaml",
        "symlinks: prompt\ndestination: ignored\ninclude:\n  - file: grand.yaml\n    wrap: grand\n    symlinks: referece\nrules:\n  - from: ../source\n    targets: [inherited]\n    to: files\n",
    );
    write(
        p,
        "config.yaml",
        "symlinks: referece\ndestination: backup\ninclude:\n  - file: nested/child.yaml\n    wrap: outer\n    symlinks: dereference\nrules: []\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    for (name, path, expected_link) in [
        ("inherited", "backup/outer/files/inherited", false),
        ("nested", "backup/outer/grand/files/nested", true),
        ("local", "backup/outer/grand/files/local", false),
    ] {
        assert_eq!(
            fs::symlink_metadata(p.join(path)).unwrap().is_symlink(),
            expected_link,
            "{name}"
        );
    }
    for name in ["inherited", "nested", "local"] {
        fs::remove_file(p.join("source").join(name)).unwrap();
    }
    let out = cli(p, &["restore", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    for name in ["inherited", "nested", "local"] {
        assert_eq!(
            fs::read_link(p.join("source").join(name)).unwrap(),
            Path::new("original")
        );
    }
}

#[test]
fn include_and_rule_metadata_overrides_apply_on_backup_and_restore() {
    use filetime::{FileTime, set_file_times};
    let dir = TempDir::new().unwrap();
    let p = dir.path();
    for name in ["root", "included", "rule"] {
        write(p, &format!("source/{name}"), name);
        let old = FileTime::from_unix_time(946684800, 0);
        set_file_times(p.join("source").join(name), old, old).unwrap();
    }
    write(
        p,
        "child.yaml",
        "symlinks: referece\ndestination: ignored\nrules:\n  - from: source\n    targets: [included]\n    to: files\n  - from: source\n    targets: [rule]\n    to: files\n    metadata: true\n",
    );
    write(
        p,
        "config.yaml",
        "symlinks: referece\nmetadata: true\ndestination: backup\ninclude:\n  - file: child.yaml\n    metadata: false\nrules:\n  - from: source\n    targets: [root]\n    to: files\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    for (name, preserved) in [("root", true), ("included", false), ("rule", true)] {
        let path = p.join("backup/files").join(name);
        let timestamp = fs::metadata(path).unwrap().modified().unwrap();
        let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(946684800);
        assert_eq!(timestamp == old, preserved, "{name}");
        fs::remove_file(p.join("source").join(name)).unwrap();
    }
    let out = cli(p, &["restore", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    for (name, preserved) in [("root", true), ("included", false), ("rule", true)] {
        let timestamp = fs::metadata(p.join("source").join(name))
            .unwrap()
            .modified()
            .unwrap();
        let old = std::time::UNIX_EPOCH + std::time::Duration::from_secs(946684800);
        assert_eq!(timestamp == old, preserved, "{name}");
    }
}

#[test]
fn invalid_include_shapes_and_wraps_fail_before_cleaning() {
    let dir = TempDir::new().unwrap();
    let p = dir.path();
    write(p, "backup/keep", "unchanged");
    for include in [
        "[child.yaml]",
        "[{file: child.yaml, wrap: ../outside}]",
        "[{file: child.yaml, wrap: /outside}]",
        "[{file: child.yaml, wrap: ''}]",
        "[{file: child.yaml, warp: typo}]",
    ] {
        write(
            p,
            "config.yaml",
            &format!(
                "symlinks: referece\ndestination: backup\nclean: true\ninclude: {include}\nrules: []\n"
            ),
        );
        let out = cli(p, &["backup", "config.yaml", "-q"], "");
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert_eq!(
            fs::read_to_string(p.join("backup/keep")).unwrap(),
            "unchanged"
        );
    }
}

#[test]
fn invalid_configuration_fails_before_side_effects() {
    let dir = TempDir::new().unwrap();
    for yaml in [
        "rules: []",
        "symlinks: referece\ndestination: backup\nrules: [bad]",
        "symlinks: referece\ndestination: backup\nrules: []\ninclude: [{file: missing.yaml}]",
        "symlinks: referece\ndestination: backup\nrules: []\ninclude: [{file: config.yaml}]",
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
        "symlinks: referece\ndestination: backup\nrules: []\ninclude: [{file: nested/child.yaml}]\n",
    );
    let denied = cli(dir.path(), &["backup", "config.yaml"], "n\n");
    assert!(denied.status.success());
    assert_eq!(text(&denied).matches("Continue?").count(), 1);
    assert!(!dir.path().join("backup").exists());
    let retried = cli(dir.path(), &["backup", "config.yaml"], "invalid\ny\n");
    assert!(retried.status.success(), "{}", text(&retried));
    assert_eq!(text(&retried).matches("Continue?").count(), 2);
    assert!(dir.path().join("backup").exists());
    let quiet = cli(dir.path(), &["backup", "config.yaml", "-q"], "");
    assert!(!text(&quiet).contains("Continue?"));
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
                        "symlinks: referece\ndestination: backup\nrules: []\ninclude: [{file: nested/child.yaml}]\n"
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
