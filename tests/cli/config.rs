use super::*;

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
