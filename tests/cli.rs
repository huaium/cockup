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
        "destination: backup\nrules: [bad]",
        "destination: backup\nrules: []\ninclude: [missing.yaml]",
        "destination: backup\nrules: []\ninclude: [config.yaml]",
    ] {
        write(dir.path(), "config.yaml", yaml);
        let out = cli(dir.path(), &["backup", "config.yaml", "-q"], "");
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert!(!dir.path().join("backup").exists());
        assert!(!out.stderr.is_empty());
    }
}

#[test]
fn included_hooks_prompt_once_and_quiet_suppresses_confirmation() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "nested/child.yaml",
        "destination: ignored\nrules: []\nhooks:\n  pre-backup:\n    - name: child\n      command: [echo, hello]\n",
    );
    write(
        dir.path(),
        "config.yaml",
        "destination: backup\nrules: []\ninclude: [nested/child.yaml]\n",
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
        "destination: ignored\nrules: []\n",
    );
    write(
        p,
        "configs/child/child.yaml",
        "destination: ignored\ninclude: [deep/grand.yaml]\nrules:\n  - from: source/*\n    targets: ['*.txt']\n    to: child\n",
    );
    write(
        p,
        "configs/root.yaml",
        "destination: ../backup\ninclude: [child/child.yaml]\nrules:\n  - from: source\n    targets: [top.txt]\n    to: top\n",
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
        "destination: backup\nrules:\n  - from: source\n    targets: [folder, broken]\n    to: files\n",
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
        Path::new("file")
    );
    assert_eq!(
        fs::read_link(p.join("backup/files/broken")).unwrap(),
        Path::new("missing")
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
        r#"destination: backup
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
        r#"destination: backup
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
        r#"destination: backup
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
        r#"destination: backup
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
        "destination: source\nclean: true\nrules:\n  - from: source\n    targets: [file]\n    to: .\n",
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
        r#"destination: backup
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
            &format!("destination: backup\nrules: []\nhooks:\n  pre-backup:\n    - {hook}\n"),
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
        "destination: backup\nrules:\n  - from: source\n    to: .\n    targets: null\n    on-start: null\nhooks: null\n",
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
        "destination: backup\nrules:\n  - from: source\n    targets: [sub/file, good]\n    to: files\n",
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
        "destination: '~/backup'\nrules:\n  - from: '~/.config'\n    targets: ['app/*', 'app/.*']\n    to: .\n",
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
    write(p, "config.yaml", include_str!("../sample/config.yaml"));
    write(
        p,
        "another_config.yaml",
        include_str!("../sample/another_config.yaml"),
    );
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
        "destination: backup\nrules:\n  - from: source\n    targets: [file]\n    to: files\n",
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
            &format!("destination: backup\nclean: true\nrules:\n  - from: source\n    {fields}\n"),
        );
        let out = cli(p, &["backup", "config.yaml", "-q"], "");
        assert_eq!(out.status.code(), Some(1), "{}", text(&out));
        assert_eq!(
            fs::read_to_string(p.join("backup/keep")).unwrap(),
            "untouched"
        );
    }
}
