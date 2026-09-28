use super::*;

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

#[test]
fn pre_backup_hook_failures_are_red_and_stop_backup() {
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
            .contains("\x1b[36;1m=> \x1b[0m\x1b[31;1mCompleted 0/2 hooks. Error: 2 hooks failed.")
    );
    assert!(!p.join("forbidden").exists());
    assert!(!p.join("finished").exists());
    assert!(!p.join("backup").exists());
    assert!(!text(&out).contains("Backup completed."));
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
fn failing_pre_hooks_stop_backup_and_restore_before_file_changes() {
    for command in ["backup", "restore"] {
        let d = TempDir::new().unwrap();
        let p = d.path();
        write(p, "source/file", "source content");
        write(p, "backup/files/file", "backup content");
        write(p, "backup/stale", "keep");
        write(
            p,
            "config.yaml",
            "symlinks: referece\nclean: true\ndestination: backup\nrules:\n  - from: source\n    targets: [file]\n    to: files\n    on-start:\n      - name: rule\n        command: [sh, -c, 'touch ../rule-ran']\nhooks:\n  pre-backup:\n    - name: fail\n      command: [sh, -c, 'exit 1']\n    - name: later\n      command: [sh, -c, 'touch later-pre-ran']\n  pre-restore:\n    - name: fail\n      command: [sh, -c, 'exit 1']\n    - name: later\n      command: [sh, -c, 'touch later-pre-ran']\n  post-backup:\n    - name: post\n      command: [sh, -c, 'touch ../post-ran']\n  post-restore:\n    - name: post\n      command: [sh, -c, 'touch ../post-ran']\n",
        );
        let out = cli(p, &[command, "config.yaml", "-q"], "");
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(
            text(&out).contains("Completed 1/2 hooks."),
            "{}",
            text(&out)
        );
        assert!(p.join("later-pre-ran").exists());
        assert!(!p.join("rule-ran").exists());
        assert!(!p.join("post-ran").exists());
        assert_eq!(
            fs::read_to_string(p.join("source/file")).unwrap(),
            "source content"
        );
        assert_eq!(
            fs::read_to_string(p.join("backup/files/file")).unwrap(),
            "backup content"
        );
        assert_eq!(fs::read_to_string(p.join("backup/stale")).unwrap(), "keep");
        assert!(!p.join("backup/.cockup-incomplete").exists());
        assert!(!p.join("backup/.cockup-symlinks.json").exists());
    }
}
