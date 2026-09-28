use super::*;

#[test]
fn cli_exposes_commands_and_version() {
    let dir = TempDir::new().unwrap();
    let out = cli(dir.path(), &["--help"], "");
    assert!(out.status.success());
    for word in [
        "backup",
        "restore",
        "hook",
        "list",
        "template",
        "completions",
    ] {
        assert!(text(&out).contains(word), "{}", text(&out));
    }
    assert!(text(&cli(dir.path(), &["--version"], "")).contains("0.2.0"));
    assert_eq!(cli(dir.path(), &["unknown"], "").status.code(), Some(2));
}

#[test]
fn completions_print_scripts_for_supported_shells() {
    let dir = TempDir::new().unwrap();
    for shell in ["bash", "zsh", "fish", "powershell", "elvish"] {
        let out = cli(dir.path(), &["completions", shell], "");
        assert!(out.status.success(), "{shell}: {}", text(&out));
        assert!(out.stderr.is_empty(), "{shell}: {}", text(&out));
        let script = String::from_utf8(out.stdout).unwrap();
        assert!(script.contains("cockup"), "{shell}: {script}");
        assert!(script.contains("backup"), "{shell}: {script}");
    }
    let unsupported = cli(dir.path(), &["completions", "unknown"], "");
    assert_eq!(unsupported.status.code(), Some(2));
}

#[test]
fn template_prints_commented_complete_valid_yaml() {
    let dir = TempDir::new().unwrap();
    let out = cli(dir.path(), &["template"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert!(out.stderr.is_empty());
    let yaml = String::from_utf8(out.stdout).unwrap();
    for field in [
        "symlinks:",
        "destination:",
        "rules:",
        "clean:",
        "metadata:",
        "include:",
        "file:",
        "wrap:",
        "from:",
        "targets:",
        "to:",
        "on-start:",
        "on-end:",
        "hooks:",
        "pre-backup:",
        "post-backup:",
        "pre-restore:",
        "post-restore:",
        "name:",
        "command:",
        "check:",
        "output:",
        "timeout:",
        "env:",
    ] {
        assert!(yaml.contains(field), "missing {field}");
    }
    assert!(yaml.contains("Required:"));
    assert!(yaml.contains("Optional:"));
    write(dir.path(), "config.yaml", &yaml);
    let loaded = cli(dir.path(), &["backup", "config.yaml", "-q"], "");
    assert!(loaded.status.success(), "{}", text(&loaded));
}

#[test]
fn quiet_accepts_only_quiet_and_q_flags() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "config.yaml",
        "symlinks: referece\ndestination: backup\nrules: []\n",
    );
    for flag in ["--quiet", "-q"] {
        let out = cli(dir.path(), &["backup", "config.yaml", flag], "");
        assert!(out.status.success(), "{}", text(&out));
    }
    for flag in ["--yes", "-y"] {
        let out = cli(dir.path(), &["backup", "config.yaml", flag], "");
        assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    }
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
    write(p, "config.yaml", include_str!("../../sample/hooks.yaml"));
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

#[test]
fn backup_and_restore_progress_use_green_with_color_enabled() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/file", "content");
    write(
        p,
        "config.yaml",
        "symlinks: referece\ndestination: backup\nrules:\n  - from: source\n    targets: [file]\n    to: files\n",
    );
    for command in ["backup", "restore"] {
        let out = Command::new(env!("CARGO_BIN_EXE_cockup"))
            .current_dir(p)
            .args([command, "config.yaml", "-q"])
            .env_remove("NO_COLOR")
            .env("FORCE_COLOR", "1")
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", text(&out));
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("\x1b[36;1m=> \x1b[0m\x1b[32;1m"),
            "{stdout}"
        );
        assert!(
            stdout.contains("\x1b[1mFile copied:\x1b[0m ")
                || stdout.contains("\x1b[1mFile existed, updating:\x1b[0m "),
            "{stdout}"
        );
        let summary = if command == "backup" {
            "Backup completed."
        } else {
            "Restore completed."
        };
        assert!(stdout.contains(summary), "{stdout}");
        assert!(stdout.contains("\x1b[0m"), "{stdout}");
    }
}

#[test]
fn backup_and_restore_report_original_operation_progress() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/one.txt", "one");
    write(p, "source/two.txt", "two");
    write(p, "source/folder/item", "nested");
    write(p, "backup/stale", "old");
    write(
        p,
        "config.yaml",
        "symlinks: referece\nclean: true\nmetadata: false\ndestination: backup\nrules:\n  - from: source\n    targets: ['*.txt', 'folder*']\n    to: files\nhooks:\n  post-backup:\n    - name: done\n      command: [sh, -c, 'true']\n",
    );
    let out = cli(p, &["backup", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    for expected in [
        "Starting backup...",
        "Clean mode enabled, will remove backup folder first if exists.",
        "Found existing backup folder, removing...",
        "Metadata preservation disabled.",
        "Target pattern matched (2 found):",
        "File copied:",
        "Folder copied:",
        "Running post-backup hooks...",
        "Running hook (1/1): done",
        "Completed 1/1 hook.",
        "Backup completed.",
    ] {
        assert!(stdout.contains(expected), "missing {expected}: {stdout}");
    }
    assert!(stdout.find("Starting backup...").unwrap() < stdout.find("File copied:").unwrap());
    assert!(
        stdout.find("File copied:").unwrap() < stdout.find("Running post-backup hooks...").unwrap()
    );
    assert!(!p.join("backup/stale").exists());
    let out = cli(p, &["restore", "config.yaml", "-q"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("Starting restore..."));
}
