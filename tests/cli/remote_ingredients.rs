use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

fn setup(home: &Path) {
    write(home, "source/file", "saved");
    write(
        home,
        "config.yaml",
        "symlinks: reference\ndestination: backup\nrules: []\ninclude:\n  - ingredient: demo\n    wrap: app\n",
    );
    write(
        home,
        "remote.yaml",
        "rules:\n  - from: ~/source\n    targets: [file]\n    to: files\n",
    );
}

fn run(home: &Path, action: &[&str], status: &str) -> Output {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/ingredient", listener.local_addr().unwrap());
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = Arc::clone(&stop);
    let body = home.join("remote.yaml");
    let calls = home.join("curl.calls");
    let status = status.to_string();
    let server = std::thread::spawn(move || {
        while !stopped.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut request = Vec::new();
                    let mut chunk = [0; 1024];
                    while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                        let count = stream.read(&mut chunk).unwrap();
                        if count == 0 {
                            break;
                        }
                        request.extend_from_slice(&chunk[..count]);
                    }
                    let flattened = String::from_utf8_lossy(&request).replace("\r\n", " | ");
                    let mut file = fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&calls)
                        .unwrap();
                    writeln!(file, "{flattened}").unwrap();
                    if status != "offline" {
                        let contents = if status == "200" {
                            fs::read(&body).unwrap()
                        } else {
                            Vec::new()
                        };
                        let header = format!(
                            "HTTP/1.1 {status} Test\r\nETag: \"demo-v1\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            contents.len()
                        );
                        stream.write_all(header.as_bytes()).unwrap();
                        stream.write_all(&contents).unwrap();
                    }
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(error) => panic!("{error}"),
            }
        }
    });
    let output = Command::new(env!("CARGO_BIN_EXE_cockup"))
        .current_dir(home)
        .env("HOME", home)
        .env("NO_COLOR", "1")
        .env("COCKUP_INGREDIENT_TEST_URL", url)
        .args(action)
        .output()
        .unwrap();
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
    output
}

#[test]
fn ingredient_fetch_is_cached_and_restore_uses_backup_snapshot() {
    let dir = TempDir::new().unwrap();
    let home = dir.path();
    setup(home);

    let backup = run(home, &["backup", "config.yaml", "-q"], "200");
    assert!(backup.status.success(), "{}", text(&backup));
    assert_eq!(
        fs::read_to_string(home.join("backup/app/files/file")).unwrap(),
        "saved"
    );
    let cache = home.join("Library/Caches/cockup/ingredients/v1");
    assert!(cache.join("demo.yaml").is_file());
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(cache.join("demo.json")).unwrap()).unwrap();
    assert!(metadata["downloaded_at_unix"].as_u64().is_some());
    assert!(
        metadata["source"]
            .as_str()
            .unwrap()
            .contains("/huaium/cockup/")
    );

    let cached = run(home, &["backup", "config.yaml", "-q"], "404");
    assert!(cached.status.success(), "{}", text(&cached));
    assert_eq!(
        fs::read_to_string(home.join("curl.calls"))
            .unwrap()
            .lines()
            .count(),
        1
    );

    fs::remove_file(home.join("source/file")).unwrap();
    fs::remove_file(cache.join("demo.yaml")).unwrap();
    let restore = run(home, &["restore", "config.yaml", "-q"], "404");
    assert!(restore.status.success(), "{}", text(&restore));
    assert_eq!(
        fs::read_to_string(home.join("source/file")).unwrap(),
        "saved"
    );
    assert_eq!(
        fs::read_to_string(home.join("curl.calls"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    let verify = run(home, &["verify", "config.yaml"], "404");
    assert!(verify.status.success(), "{}", text(&verify));
    assert_eq!(
        fs::read_to_string(home.join("curl.calls"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[test]
fn stale_ingredient_uses_conditional_refresh_and_preserves_download_date_on_304() {
    let dir = TempDir::new().unwrap();
    let home = dir.path();
    setup(home);
    assert!(
        run(home, &["backup", "config.yaml", "-q"], "200")
            .status
            .success()
    );
    let metadata_path = home.join("Library/Caches/cockup/ingredients/v1/demo.json");
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).unwrap()).unwrap();
    let downloaded = metadata["downloaded_at_unix"].as_u64().unwrap();
    metadata["checked_at_unix"] = 0.into();
    fs::write(&metadata_path, serde_json::to_vec(&metadata).unwrap()).unwrap();

    let refreshed = run(home, &["backup", "config.yaml", "-q"], "304");
    assert!(refreshed.status.success(), "{}", text(&refreshed));
    let checked: serde_json::Value =
        serde_json::from_slice(&fs::read(metadata_path).unwrap()).unwrap();
    assert_eq!(checked["downloaded_at_unix"].as_u64(), Some(downloaded));
    assert!(checked["checked_at_unix"].as_u64().unwrap() >= downloaded);
    let calls = fs::read_to_string(home.join("curl.calls")).unwrap();
    assert_eq!(calls.lines().count(), 2);
    assert!(
        calls
            .to_ascii_lowercase()
            .contains("if-none-match: \"demo-v1\"")
    );
}

#[test]
fn missing_or_invalid_remote_ingredient_fails_before_backup_changes() {
    for (status, yaml) in [
        ("404", "rules: []\n"),
        ("200", "rules: []\nhooks:\n  pre-backup: []\n"),
        (
            "200",
            "rules:\n  - from: /etc\n    targets: [hosts]\n    to: files\n",
        ),
        (
            "200",
            "rules:\n  - from: ~/source\n    targets: [file]\n    to: ../outside\n",
        ),
    ] {
        let dir = TempDir::new().unwrap();
        let home = dir.path();
        setup(home);
        write(home, "remote.yaml", yaml);
        let backup = run(home, &["backup", "config.yaml", "-q"], status);
        assert_eq!(backup.status.code(), Some(1), "{}", text(&backup));
        assert!(!home.join("backup").exists());
        assert!(
            !home
                .join("Library/Caches/cockup/ingredients/v1/demo.yaml")
                .exists()
        );
    }
}

#[test]
fn dry_run_fetches_without_writing_cache() {
    let dir = TempDir::new().unwrap();
    let home = dir.path();
    setup(home);
    let preview = run(home, &["backup", "config.yaml", "-q", "--dry-run"], "200");
    assert!(preview.status.success(), "{}", text(&preview));
    assert!(!home.join("backup").exists());
    assert!(!home.join("Library/Caches/cockup/ingredients/v1").exists());
    assert!(
        run(home, &["backup", "config.yaml", "-q"], "200")
            .status
            .success()
    );
    assert_eq!(
        fs::read_to_string(home.join("curl.calls"))
            .unwrap()
            .lines()
            .count(),
        2
    );
}

#[test]
fn stale_cache_survives_network_failure_but_not_a_missing_remote_file() {
    let dir = TempDir::new().unwrap();
    let home = dir.path();
    setup(home);
    assert!(
        run(home, &["backup", "config.yaml", "-q"], "200")
            .status
            .success()
    );
    let metadata_path = home.join("Library/Caches/cockup/ingredients/v1/demo.json");
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).unwrap()).unwrap();
    metadata["checked_at_unix"] = 0.into();
    fs::write(&metadata_path, serde_json::to_vec(&metadata).unwrap()).unwrap();

    let offline = run(home, &["backup", "config.yaml", "-q"], "offline");
    assert!(offline.status.success(), "{}", text(&offline));
    assert!(text(&offline).contains("using cached copy"));
    let missing = run(home, &["backup", "config.yaml", "-q"], "404");
    assert_eq!(missing.status.code(), Some(1), "{}", text(&missing));
    assert!(text(&missing).contains("not found"));
}

#[test]
fn include_source_must_be_unique_and_ingredient_names_cannot_escape_cache() {
    for include in [
        "{file: other.yaml, ingredient: demo}",
        "{wrap: app}",
        "{ingredient: ../escape}",
    ] {
        let dir = TempDir::new().unwrap();
        let home = dir.path();
        setup(home);
        write(
            home,
            "config.yaml",
            &format!("symlinks: reference\ndestination: backup\nrules: []\ninclude: [{include}]\n"),
        );
        let output = run(home, &["backup", "config.yaml", "-q"], "200");
        assert_eq!(output.status.code(), Some(1), "{}", text(&output));
        assert!(!home.join("backup").exists());
        assert!(!home.join("curl.calls").exists());
    }
}

#[test]
fn ingredient_update_and_status_manage_the_cache_without_a_config() {
    let dir = TempDir::new().unwrap();
    let home = dir.path();
    setup(home);

    let empty = run(home, &["ingredient", "status"], "offline");
    assert!(empty.status.success(), "{}", text(&empty));
    assert!(text(&empty).contains("No cached ingredients"));
    assert!(!home.join("curl.calls").exists());

    let updated = run(home, &["ingredient", "update", "demo"], "200");
    assert!(updated.status.success(), "{}", text(&updated));
    let cache = home.join("Library/Caches/cockup/ingredients/v1");
    assert!(cache.join("demo.yaml").is_file());

    let status = run(home, &["ingredient", "status", "demo"], "offline");
    assert!(status.status.success(), "{}", text(&status));
    for expected in ["demo", "Downloaded:", "Checked:", "Source:", "ETag:"] {
        assert!(text(&status).contains(expected), "{}", text(&status));
    }
    assert_eq!(
        fs::read_to_string(home.join("curl.calls"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    let all = run(home, &["ingredient", "status"], "offline");
    assert!(all.status.success(), "{}", text(&all));
    assert!(text(&all).contains("demo:"));
    let missing = run(home, &["ingredient", "status", "missing"], "offline");
    assert_eq!(missing.status.code(), Some(1));
    assert_eq!(
        fs::read_to_string(home.join("curl.calls"))
            .unwrap()
            .lines()
            .count(),
        1
    );

    let sidecar = cache.join("demo.json");
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(&sidecar).unwrap()).unwrap();
    metadata["downloaded_at_unix"] = 0.into();
    metadata["checked_at_unix"] = 0.into();
    fs::write(sidecar, serde_json::to_vec(&metadata).unwrap()).unwrap();
    let dated = run(home, &["ingredient", "status", "demo"], "offline");
    assert!(dated.status.success(), "{}", text(&dated));
    assert!(text(&dated).contains("1970-01-01 00:00:00 UTC"));
}

#[test]
fn manual_update_forces_a_check_and_preserves_cached_yaml_on_304_or_failure() {
    use std::os::unix::fs::MetadataExt;

    let dir = TempDir::new().unwrap();
    let home = dir.path();
    setup(home);
    assert!(
        run(home, &["ingredient", "update", "demo"], "200")
            .status
            .success()
    );
    let cache = home.join("Library/Caches/cockup/ingredients/v1");
    let yaml_path = cache.join("demo.yaml");
    let metadata_path = cache.join("demo.json");
    let inode = fs::metadata(&yaml_path).unwrap().ino();
    let original = fs::read(&yaml_path).unwrap();
    let before: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).unwrap()).unwrap();

    let unchanged = run(home, &["ingredient", "update", "demo"], "304");
    assert!(unchanged.status.success(), "{}", text(&unchanged));
    assert_eq!(fs::metadata(&yaml_path).unwrap().ino(), inode);
    let after: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata_path).unwrap()).unwrap();
    assert_eq!(after["downloaded_at_unix"], before["downloaded_at_unix"]);
    assert_eq!(after["source"], before["source"]);
    assert_eq!(after["etag"], before["etag"]);
    assert_eq!(
        fs::read_to_string(home.join("curl.calls"))
            .unwrap()
            .lines()
            .count(),
        2
    );

    for failure in ["404", "offline"] {
        let result = run(home, &["ingredient", "update", "demo"], failure);
        assert_eq!(result.status.code(), Some(1), "{}", text(&result));
        assert_eq!(fs::read(&yaml_path).unwrap(), original);
    }
}

#[test]
fn ingredient_delete_removes_one_pair_and_clean_removes_the_cockup_cache_root() {
    let dir = TempDir::new().unwrap();
    let home = dir.path();
    setup(home);
    let cache = home.join("Library/Caches/cockup/ingredients/v1");
    write(home, "Library/Caches/cockup/keep", "remove me");
    write(home, "Library/Caches/other/keep", "untouched");
    for name in ["demo", "zed"] {
        let output = run(home, &["ingredient", "update", name], "200");
        assert!(output.status.success(), "{}", text(&output));
    }

    let deleted = run(home, &["ingredient", "delete", "demo"], "offline");
    assert!(deleted.status.success(), "{}", text(&deleted));
    assert!(!cache.join("demo.yaml").exists());
    assert!(!cache.join("demo.json").exists());
    assert!(cache.join("zed.yaml").is_file());
    assert!(cache.join("zed.json").is_file());
    assert_eq!(
        run(home, &["ingredient", "delete", "demo"], "offline")
            .status
            .code(),
        Some(1)
    );
    assert_eq!(
        run(home, &["ingredient", "delete", "../zed"], "offline")
            .status
            .code(),
        Some(1)
    );

    let cleaned = run(home, &["ingredient", "clean"], "offline");
    assert!(cleaned.status.success(), "{}", text(&cleaned));
    assert!(!home.join("Library/Caches/cockup").exists());
    assert_eq!(
        fs::read_to_string(home.join("Library/Caches/other/keep")).unwrap(),
        "untouched"
    );
    assert!(
        run(home, &["ingredient", "clean"], "offline")
            .status
            .success()
    );
}

#[test]
fn ingredient_clean_unlinks_nested_cache_symlinks_without_following_them() {
    let dir = TempDir::new().unwrap();
    let home = dir.path();
    setup(home);
    write(home, "external/demo.yaml", "keep");
    let cache = home.join("Library/Caches/cockup/ingredients/v1");
    fs::create_dir_all(cache.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(home.join("external"), &cache).unwrap();

    let deleted = run(home, &["ingredient", "delete", "demo"], "offline");
    assert_eq!(deleted.status.code(), Some(1), "{}", text(&deleted));
    let cleaned = run(home, &["ingredient", "clean"], "offline");
    assert!(cleaned.status.success(), "{}", text(&cleaned));
    assert!(!cache.exists());
    assert_eq!(
        fs::read_to_string(home.join("external/demo.yaml")).unwrap(),
        "keep"
    );
}

#[test]
fn ingredient_clean_refuses_a_symlinked_cockup_cache_root() {
    let dir = TempDir::new().unwrap();
    let home = dir.path();
    setup(home);
    write(home, "external/keep", "untouched");
    let root = home.join("Library/Caches/cockup");
    fs::create_dir_all(root.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(home.join("external"), &root).unwrap();
    let result = run(home, &["ingredient", "clean"], "offline");
    assert_eq!(result.status.code(), Some(1), "{}", text(&result));
    assert!(text(&result).contains("not a directory"));
    assert!(fs::symlink_metadata(root).unwrap().file_type().is_symlink());
    assert_eq!(
        fs::read_to_string(home.join("external/keep")).unwrap(),
        "untouched"
    );
}

#[test]
fn ingredient_delete_accepts_multiple_names_and_continues_after_a_missing_entry() {
    let dir = TempDir::new().unwrap();
    let home = dir.path();
    setup(home);
    let cache = home.join("Library/Caches/cockup/ingredients/v1");
    for name in ["demo", "zed"] {
        assert!(
            run(home, &["ingredient", "update", name], "200")
                .status
                .success()
        );
    }

    let both = run(home, &["ingredient", "delete", "demo", "zed"], "offline");
    assert!(both.status.success(), "{}", text(&both));
    for name in ["demo", "zed"] {
        assert!(!cache.join(format!("{name}.yaml")).exists());
        assert!(!cache.join(format!("{name}.json")).exists());
    }

    assert!(
        run(home, &["ingredient", "update", "demo"], "200")
            .status
            .success()
    );
    let invalid = run(home, &["ingredient", "delete", "demo", "../zed"], "offline");
    assert_eq!(invalid.status.code(), Some(1));
    assert!(cache.join("demo.yaml").exists());

    let partial = run(
        home,
        &["ingredient", "delete", "missing", "demo"],
        "offline",
    );
    assert_eq!(partial.status.code(), Some(1), "{}", text(&partial));
    assert!(!cache.join("demo.yaml").exists());
}

#[test]
fn documented_ingredient_sample_backs_up_verifies_and_restores() {
    let dir = TempDir::new().unwrap();
    let home = dir.path();
    let sample_config = include_str!("../../sample/ingredient.yaml");
    let sample_source = include_str!("../../sample/src/homes/ghostty/.config/ghostty/config");
    let app_support = include_str!(
        "../../sample/src/homes/ghostty/Library/Application Support/com.mitchellh.ghostty/sample.txt"
    );
    write(home, "config.yaml", sample_config);
    write(home, ".config/ghostty/config", sample_source);
    write(
        home,
        "Library/Application Support/com.mitchellh.ghostty/sample.txt",
        app_support,
    );
    write(
        home,
        "remote.yaml",
        include_str!("../../ingredients/library/ghostty.yaml"),
    );

    let backup = run(home, &["backup", "config.yaml", "-q"], "200");
    assert!(backup.status.success(), "{}", text(&backup));
    assert_eq!(
        fs::read_to_string(home.join("dst/ingredient/ghostty/.config/ghostty/config")).unwrap(),
        sample_source
    );
    assert_eq!(
        fs::read_to_string(home.join(
            "dst/ingredient/ghostty/Library/Application Support/com.mitchellh.ghostty/sample.txt"
        ))
        .unwrap(),
        app_support
    );
    let verify = run(home, &["verify", "config.yaml"], "offline");
    assert!(verify.status.success(), "{}", text(&verify));

    fs::remove_file(home.join(".config/ghostty/config")).unwrap();
    let restore = run(home, &["restore", "config.yaml", "-q"], "offline");
    assert!(restore.status.success(), "{}", text(&restore));
    assert_eq!(
        fs::read_to_string(home.join(".config/ghostty/config")).unwrap(),
        sample_source
    );
    assert_eq!(
        fs::read_to_string(home.join("curl.calls"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}
