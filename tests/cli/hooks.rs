use super::*;

#[test]
fn shell_strings_expand_environment_and_preserve_quoted_paths() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(
        p,
        "config.yaml",
        r#"destination: backup
symlinks: reference
rules: []
hooks:
  pre-backup:
    - name: shell
      command: |
        printf '%s' "$VALUE" > "file with spaces"
        printf '%s' "${OPTIONAL:-fallback}" > fallback
      check: 'test "$(cat "file with spaces")" = "$VALUE" && test "$(cat fallback)" = fallback'
      env:
        VALUE: '$(touch injected); echo bad'
"#,
    );
    let out = cli(p, &["hook", "config.yaml", "-a", "-n", "shell"], "");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(p.join("file with spaces")).unwrap(),
        "$(touch injected); echo bad"
    );
    assert!(!p.join("injected").exists());
}

#[test]
fn shell_unset_variables_fail_commands_and_checks_but_continue_independent_hooks() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(
        p,
        "config.yaml",
        r#"destination: backup
symlinks: reference
rules: []
hooks:
  pre-backup:
    - name: unset-command
      command: 'unset COCKUP_UNSET; printf "%s" "$COCKUP_UNSET" > command-marker'
      check: 'touch forbidden-check'
    - name: unset-check
      command: 'touch command-ran'
      check: 'unset COCKUP_UNSET; test "$COCKUP_UNSET" = value'
    - name: later
      command: 'touch later-marker'
"#,
    );
    let out = cli(p, &["hook", "config.yaml", "-a"], "1,2,3\n");
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains("command: exited"));
    assert!(text(&out).contains("check: exited"));
    assert!(!p.join("forbidden-check").exists());
    assert!(!p.join("command-marker").exists());
    assert!(p.join("command-ran").is_file());
    assert!(p.join("later-marker").is_file());
}

#[test]
fn shell_command_and_check_timeouts_kill_only_the_direct_shell() {
    for field in ["command", "check"] {
        let d = TempDir::new().unwrap();
        let p = d.path();
        let script = "(sleep 2; touch escaped) & echo $! > descendant-pid; wait";
        let commands = if field == "command" {
            format!("command: '{script}'\n      check: 'touch forbidden-check'")
        } else {
            format!("command: 'true'\n      check: '{script}'")
        };
        write(
            p,
            "config.yaml",
            &format!(
                "destination: backup\nsymlinks: reference\nrules: []\nhooks:\n  pre-backup:\n    - name: timeout\n      {commands}\n      output: true\n      timeout: 1\n"
            ),
        );
        let started = std::time::Instant::now();
        let out = cli(p, &["hook", "config.yaml", "-a", "-n", "timeout"], "");
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(text(&out).contains("timed out"));
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        std::thread::sleep(std::time::Duration::from_millis(1300));
        assert!(
            p.join("escaped").exists(),
            "{field} descendant should remain independent of direct-child cleanup"
        );
        assert!(!p.join("forbidden-check").exists());
    }
}

#[test]
#[cfg(target_os = "macos")]
fn shell_hook_can_read_from_a_real_terminal() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(
        p,
        "config.yaml",
        "destination: backup\nsymlinks: reference\nrules: []\nhooks:\n  pre-backup:\n    - name: terminal\n      command: 'touch ready; read value; printf \"%s\" \"$value\" > answer'\n      output: true\n      timeout: 2\n",
    );
    let mut child = Command::new("/usr/bin/script")
        .args([
            "-q",
            "/dev/null",
            env!("CARGO_BIN_EXE_cockup"),
            "hook",
            "config.yaml",
            "-a",
            "-n",
            "terminal",
        ])
        .current_dir(p)
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !p.join("ready").exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    input.write_all(b"hello terminal\n").unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(p.join("answer").exists(), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(p.join("answer")).unwrap(),
        "hello terminal"
    );
}

#[test]
#[cfg(target_os = "macos")]
fn terminal_ctrl_c_waits_for_the_hook_and_skips_checks_and_later_work() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(
        p,
        "config.yaml",
        r#"destination: backup
symlinks: reference
rules: []
hooks:
  pre-backup:
    - name: interrupt
      command: |
        trap 'sleep 0.2; touch handled; exit 0' INT
        touch ready
        while :; do sleep 1; done
      check: 'touch forbidden-check'
      output: true
      timeout: 5
    - name: later
      command: 'touch forbidden-later'
"#,
    );
    let mut child = Command::new("/usr/bin/script")
        .args([
            "-q",
            "/dev/null",
            env!("CARGO_BIN_EXE_cockup"),
            "backup",
            "config.yaml",
            "-a",
        ])
        .current_dir(p)
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !p.join("ready").exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let mut input = child.stdin.take().unwrap();
    input.write_all(&[3]).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(p.join("ready").exists(), "{}", text(&out));
    assert!(
        p.join("handled").exists(),
        "Cockup did not wait for the hook: {}",
        text(&out)
    );
    assert!(text(&out).contains("interrupted"), "{}", text(&out));
    assert!(!p.join("forbidden-check").exists());
    assert!(!p.join("forbidden-later").exists());
    assert!(!p.join("backup").exists());
}

#[test]
fn hook_arrays_and_empty_shell_strings_are_rejected_before_any_execution() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    for (fields, message) in [
        ("command: [echo, hello]", "arrays are no longer supported"),
        (
            "command: 'true'\n      check: [test, -d, .]",
            "arrays are no longer supported",
        ),
        ("command: ''", "nonempty shell string"),
        ("command: '   '", "nonempty shell string"),
        ("command: 'true'\n      check: ''", "nonempty shell string"),
        ("command: 42", "nonempty shell string"),
    ] {
        write(
            p,
            "config.yaml",
            &format!(
                "destination: backup\nsymlinks: reference\nrules: []\nhooks:\n  pre-backup:\n    - name: first\n      command: 'touch forbidden'\n    - name: invalid\n      {fields}\n"
            ),
        );
        let out = cli(p, &["backup", "config.yaml", "-a"], "");
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(text(&out).contains(message), "{}", text(&out));
        assert!(!p.join("forbidden").exists());
        assert!(!p.join("backup").exists());
    }
}

#[test]
fn standalone_hooks_use_lifecycle_directories_for_every_stage() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    fs::create_dir(p.join("backup")).unwrap();
    write(
        p,
        "config.yaml",
        r#"destination: backup
symlinks: reference
rules:
  - from: source
    to: files
    targets: []
    on-start:
      - name: start
        command: 'pwd > start'
        check: 'test -s start'
    on-end:
      - name: end
        command: 'pwd > end'
hooks:
  pre-backup:
    - name: pre-backup
      command: 'pwd > pre-backup'
  pre-restore:
    - name: pre-restore
      command: 'pwd > pre-restore'
  post-backup:
    - name: post-backup
      command: 'pwd > post-backup'
  post-restore:
    - name: post-restore
      command: 'pwd > post-restore'
"#,
    );
    for (name, base) in [
        ("pre-backup", p),
        ("pre-restore", p),
        ("post-backup", p.join("backup").as_path()),
        ("post-restore", p.join("backup").as_path()),
        ("start", p.join("backup").as_path()),
        ("end", p.join("backup").as_path()),
    ] {
        let out = cli(p, &["hook", "config.yaml", "-a", "-n", name], "");
        assert!(out.status.success(), "{}", text(&out));
        assert_eq!(
            fs::read_to_string(base.join(name)).unwrap().trim(),
            fs::canonicalize(base).unwrap().to_str().unwrap()
        );
    }
}

#[test]
fn standalone_hooks_prompt_before_creating_missing_directories() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(
        p,
        "config.yaml",
        r#"destination: nested/backup
symlinks: reference
rules: []
hooks:
  pre-backup:
    - name: pre
      command: 'touch pre-marker'
  post-backup:
    - name: post
      command: 'touch post-marker'
      check: 'test -f post-marker'
    - name: second
      command: 'touch second-marker'
    - name: custom
      cwd: custom/work
      command: 'touch custom-marker'
"#,
    );
    let pre = cli(p, &["hook", "config.yaml", "-a", "-n", "pre"], "");
    assert!(pre.status.success(), "{}", text(&pre));
    assert!(!text(&pre).contains("Create working directory?"));
    assert!(!p.join("nested").exists());
    let declined = cli(p, &["hook", "config.yaml", "-a", "-n", "post"], "n\n");
    assert!(declined.status.success(), "{}", text(&declined));
    assert!(text(&declined).contains("cancelled"));
    assert!(!p.join("nested").exists());
    let accepted = cli(p, &["hook", "config.yaml", "-a"], "2,3\ninvalid\ny\n");
    assert!(accepted.status.success(), "{}", text(&accepted));
    assert_eq!(
        text(&accepted).matches("Create working directory?").count(),
        2
    );
    assert!(p.join("nested/backup/post-marker").is_file());
    assert!(p.join("nested/backup/second-marker").is_file());
    assert!(!p.join("post-marker").exists());
    let custom = cli(p, &["hook", "config.yaml", "-a", "-n", "custom"], "y\n");
    assert!(custom.status.success(), "{}", text(&custom));
    assert!(p.join("custom/work/custom-marker").is_file());
    assert!(!p.join("nested/backup/custom-marker").exists());
}

#[test]
fn standalone_hooks_reject_files_and_do_not_run_any_hook_on_declined_creation() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "backup", "keep");
    write(
        p,
        "config.yaml",
        "destination: backup\nsymlinks: reference\nrules: []\nhooks:\n  pre-backup:\n    - name: pre\n      command: 'touch must-not-run'\n  post-backup:\n    - name: post\n      command: 'touch post-marker'\n",
    );
    let out = cli(p, &["hook", "config.yaml", "-a"], "1,2\n");
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains("not a directory"));
    assert!(!p.join("must-not-run").exists());
    assert_eq!(fs::read_to_string(p.join("backup")).unwrap(), "keep");
    write(
        p,
        "config.yaml",
        "destination: missing\nsymlinks: reference\nrules: []\nhooks:\n  pre-backup:\n    - name: pre\n      command: 'touch must-not-run'\n  post-backup:\n    - name: post\n      command: 'true'\n",
    );
    let out = cli(p, &["hook", "config.yaml", "-a"], "1,2\nn\n");
    assert!(out.status.success(), "{}", text(&out));
    assert!(!p.join("must-not-run").exists());
    assert!(!p.join("missing").exists());
}

#[test]
fn hook_cwd_resolves_against_its_own_yaml_and_applies_to_checks() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    fs::create_dir_all(p.join("included/work space")).unwrap();
    write(
        p,
        "config.yaml",
        "destination: backup\nsymlinks: reference\nrules: []\ninclude: [{file: included/hooks.yaml}]\n",
    );
    write(
        p,
        "included/hooks.yaml",
        r#"rules: []
hooks:
  pre-backup:
    - name: custom
      cwd: work space
      command: 'printf custom > marker'
      check: 'test "$(cat marker)" = custom'
    - name: default
      command: 'printf default > default-marker'
"#,
    );
    let out = cli(p, &["hook", "config.yaml", "-a"], "1,2\n");
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(p.join("included/work space/marker")).unwrap(),
        "custom"
    );
    assert!(p.join("default-marker").is_file());
    assert!(!p.join("marker").exists());
}

#[test]
fn hook_cwd_expands_home_for_rule_and_lifecycle_hooks_and_dry_run() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "home/work/file", "content");
    write(
        p,
        "config.yaml",
        r#"destination: backup
symlinks: reference
rules:
  - from: home/work
    to: files
    targets: [file]
    on-end:
      - name: rule
        cwd: ~/work
        command: 'printf rule > rule-marker'
        check: 'test -f rule-marker'
hooks:
  post-backup:
    - name: home
      cwd: '~'
      command: 'printf home > home-marker'
      check: 'test -f home-marker'
    - name: absolute
      cwd: ABSOLUTE
      command: 'test -f file'
"#
        .replace(
            "ABSOLUTE",
            &serde_json::to_string(&p.join("home/work")).unwrap(),
        )
        .as_str(),
    );
    let run = |dry: bool| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_cockup"));
        cmd.current_dir(p)
            .env("HOME", p.join("home"))
            .env("NO_COLOR", "1")
            .args(["backup", "config.yaml", "-a"]);
        if dry {
            cmd.arg("--dry-run");
        }
        cmd.output().unwrap()
    };
    let preview = run(true);
    assert!(preview.status.success(), "{}", text(&preview));
    assert!(text(&preview).contains(&format!(
        "Would run hook `rule` in: {}",
        p.join("home/work").display()
    )));
    assert!(!p.join("home/home-marker").exists());
    assert!(!p.join("backup").exists());
    let out = run(false);
    assert!(out.status.success(), "{}", text(&out));
    assert!(p.join("home/work/rule-marker").is_file());
    assert!(p.join("home/home-marker").is_file());
    assert!(!p.join("backup/home-marker").exists());
}

#[test]
fn invalid_hook_cwd_fails_before_execution_and_independent_hooks_continue() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "not-directory", "file");
    for cwd in ["missing", "not-directory", "''"] {
        write(
            p,
            "config.yaml",
            &format!(
                "destination: backup\nsymlinks: reference\nrules: []\nhooks:\n  pre-backup:\n    - name: invalid\n      cwd: {cwd}\n      command: 'touch must-not-run'\n      check: 'touch check-must-not-run'\n    - name: independent\n      command: 'touch continued'\n"
            ),
        );
        let out = cli(p, &["backup", "config.yaml", "-a"], "");
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(text(&out).contains("cwd"));
        assert!(!p.join("must-not-run").exists());
        assert!(!p.join("check-must-not-run").exists());
        assert!(!p.join("backup").exists());
        if cwd != "''" {
            assert!(p.join("continued").exists());
        }
    }
}

#[test]
fn hooks_run_in_order_with_checks_environment_and_working_directories() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(p, "source/file", "content");
    write(
        p,
        "config.yaml",
        r#"symlinks: reference
destination: backup
rules:
  - from: source
    to: files
    targets: [file]
    on-start:
      - name: start
        command: 'printf start >> ../order; test ! -f files/file'
    on-end:
      - name: end
        command: 'test -f files/file && printf end >> ../order'
hooks:
  pre-backup:
    - name: pre
      command: 'printf "$TOKEN" > order'
      check: 'test "$(cat order)" = pre'
      env: {TOKEN: pre, NUMBER: 12}
  post-backup:
    - name: post
      command: 'printf post >> ../order'
      check: 'test "$(cat ../order)" = prestartendpost'
"#,
    );
    let out = cli(p, &["backup", "config.yaml", "-a"], "");
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
        r#"symlinks: reference
destination: backup
rules: []
hooks:
  pre-backup:
    - name: bad
      command: 'printf main'
      check: 'exit 1'
      output: true
    - name: good
      command: 'printf good > marker'
"#,
    );
    let out = cli(p, &["hook", "config.yaml", "-a", "-n", "bad"], "");
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains("Completed 0/1 hook. Error: 1 hook failed."));
    assert!(text(&out).contains("check"));
    assert!(!p.join("marker").exists());
    let out = cli(p, &["hook", "config.yaml", "--approve-hooks"], "1,2\n");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(fs::read_to_string(p.join("marker")).unwrap(), "good");
    assert!(text(&out).contains("Completed 1/2 hooks"));
    assert_eq!(
        cli(p, &["hook", "config.yaml", "-a", "-n", "missing"], "")
            .status
            .code(),
        Some(1)
    );
    assert_eq!(
        cli(p, &["hook", "config.yaml", "-a"], "bogus\n")
            .status
            .code(),
        Some(1)
    );
    fs::remove_file(p.join("marker")).unwrap();
    let retried = cli(p, &["hook", "config.yaml", "-a"], "bogus\n2\n");
    assert!(retried.status.success(), "{}", text(&retried));
    assert_eq!(text(&retried).matches("Select hooks").count(), 2);
    assert_eq!(fs::read_to_string(p.join("marker")).unwrap(), "good");
}

#[test]
fn timed_out_commands_and_checks_fail_and_skip_dependent_work() {
    let d = TempDir::new().unwrap();
    let p = d.path();
    write(
        p,
        "config.yaml",
        r#"symlinks: reference
destination: backup
rules: []
hooks:
  pre-backup:
    - name: command-timeout
      command: 'echo $$ > pid; exec sleep 2'
      check: 'touch should-not-exist'
      timeout: 1
    - name: check-timeout
      command: 'true'
      check: 'sleep 2'
      timeout: 1
"#
        .replace("[true]", "['true']")
        .as_str(),
    );
    for name in ["command-timeout", "check-timeout"] {
        let out = cli(p, &["hook", "config.yaml", "-a", "-n", name], "");
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
        r#"symlinks: reference
destination: backup
rules:
  - from: source
    targets: ['missing*', file]
    to: files
hooks:
  pre-backup:
    - name: failed-command
      command: 'exit 1'
      check: 'touch forbidden'
    - name: missing-command
      command: 'cockup-test-nonexistent-command'
  post-backup:
    - name: finish
      command: 'touch ../finished'
"#,
    );
    let out = Command::new(env!("CARGO_BIN_EXE_cockup"))
        .current_dir(p)
        .args(["backup", "config.yaml", "-a"])
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
        "seed.yaml",
        "destination: backup\nsymlinks: reference\nrules: []\n",
    );
    assert!(cli(p, &["backup", "seed.yaml"], "").status.success());
    write(
        p,
        "config.yaml",
        r#"symlinks: reference
destination: backup
rules:
  - from: source
    to: files
    targets: [file]
hooks:
  pre-restore:
    - name: before
      command: 'test ! -e source/file; printf before > order; echo hidden'
  post-restore:
    - name: after
      command: 'test -f ../source/file && printf after >> ../order; echo visible'
      check: 'echo checked; test "$(cat ../order)" = beforeafter'
      output: true
"#,
    );
    let out = cli(p, &["restore", "config.yaml", "-a"], "");
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
        "name: invalid\n      command: 'echo hello'\n      check: []",
        "name: invalid\n      command: 'echo hello'\n      timeout: -1",
    ] {
        write(
            p,
            "config.yaml",
            &format!(
                "symlinks: reference\ndestination: backup\nrules: []\nhooks:\n  pre-backup:\n    - {hook}\n"
            ),
        );
        assert_eq!(
            cli(p, &["backup", "config.yaml", "-a"], "").status.code(),
            Some(1)
        );
        assert!(!p.join("backup").exists());
    }
    write(
        p,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules:\n  - from: source\n    to: .\n    targets: null\n    on-start: null\nhooks: null\n",
    );
    assert!(
        cli(p, &["backup", "config.yaml", "-a"], "")
            .status
            .success()
    );
    assert_eq!(
        cli(p, &["hook", "config.yaml", "-a"], "").status.code(),
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
            "seed.yaml",
            "destination: backup\nsymlinks: reference\nrules: []\n",
        );
        assert!(cli(p, &["backup", "seed.yaml"], "").status.success());
        let manifest_before = fs::read(p.join("backup/.cockup-symlinks.json")).unwrap();
        write(
            p,
            "config.yaml",
            "symlinks: reference\nclean: true\ndestination: backup\nrules:\n  - from: source\n    targets: [file]\n    to: files\n    on-start:\n      - name: rule\n        command: 'touch ../rule-ran'\nhooks:\n  pre-backup:\n    - name: fail\n      command: 'exit 1'\n    - name: later\n      command: 'touch later-pre-ran'\n  pre-restore:\n    - name: fail\n      command: 'exit 1'\n    - name: later\n      command: 'touch later-pre-ran'\n  post-backup:\n    - name: post\n      command: 'touch ../post-ran'\n  post-restore:\n    - name: post\n      command: 'touch ../post-ran'\n",
        );
        let out = cli(p, &[command, "config.yaml", "-a"], "");
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
        assert_eq!(
            fs::read(p.join("backup/.cockup-symlinks.json")).unwrap(),
            manifest_before
        );
    }
}
