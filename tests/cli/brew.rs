use super::*;

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
    let colored = Command::new(env!("CARGO_BIN_EXE_cockup"))
        .current_dir(p)
        .args(["list", "alpha"])
        .env("PATH", p)
        .env_remove("NO_COLOR")
        .env("FORCE_COLOR", "1")
        .output()
        .unwrap();
    assert!(colored.status.success(), "{}", text(&colored));
    assert!(
        String::from_utf8_lossy(&colored.stdout)
            .contains("\x1b[32;1malpha:\x1b[0m\n  ~/empty\n  ~/alpha\n"),
        "{}",
        text(&colored)
    );
    assert!(run(&["list", "zeta"]).status.success());
    fs::remove_file(p.join("brew")).unwrap();
    let out = run(&["list"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out).contains("Homebrew"));
}
