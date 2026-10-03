use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    time::{Duration, Instant},
};

fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cockup"))
        .current_dir(home)
        .env("HOME", home)
        .env("NO_COLOR", "1")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn cache_commands_manage_catalog_and_ingredients_offline() {
    let dir = TempDir::new().unwrap();
    let home = dir.path();
    let cache = home.join("Library/Caches/cockup/ingredients/v1");
    fs::create_dir_all(&cache).unwrap();
    fs::write(
        cache.join("demo.yaml"),
        "rules:\n  - from: ~/config\n    targets: [file]\n    to: files\n",
    )
    .unwrap();
    fs::write(cache.join("demo.json"), br#"{"downloaded_at_unix":0,"checked_at_unix":0,"source_ref":"main","source":"","etag":null}"#).unwrap();
    fs::write(cache.join(".catalog.json"), br#"{"version":1,"source":"https://api.github.com/repos/huaium/cockup/contents/ingredients/library?ref=main","downloaded_at_unix":0,"checked_at_unix":0,"etag":null,"names":["demo"]}"#).unwrap();

    for (args, expected) in [
        (&["cache", "status"][..], "Catalog:"),
        (&["cache", "list"][..], "demo"),
        (&["cache", "show", "demo"][..], "Downloaded:"),
    ] {
        let output = run(home, args);
        assert!(output.status.success(), "{}", text(&output));
        assert!(text(&output).contains(expected), "{}", text(&output));
    }
    let deleted = run(home, &["cache", "delete", "--catalog"]);
    assert!(deleted.status.success(), "{}", text(&deleted));
    assert!(!cache.join(".catalog.json").exists());
    assert!(cache.join("demo.yaml").exists());
    let removed = run(home, &["cache", "delete", "demo"]);
    assert!(removed.status.success(), "{}", text(&removed));
    assert!(!cache.join("demo.yaml").exists());
    assert!(!cache.join("demo.json").exists());
}

#[test]
fn cache_delete_all_ingredients_keeps_catalog_and_unlinks_cache_files() {
    let dir = TempDir::new().unwrap();
    let home = dir.path();
    let cache = home.join("Library/Caches/cockup/ingredients/v1");
    fs::create_dir_all(&cache).unwrap();
    for file in ["demo.yaml", "demo.json", "orphan.yaml", "broken.json"] {
        fs::write(cache.join(file), "invalid or incomplete cache").unwrap();
    }
    fs::write(cache.join(".catalog.json"), "catalog contents").unwrap();
    fs::write(cache.join("keep.txt"), "unrelated").unwrap();
    fs::write(home.join("external.yaml"), "keep target").unwrap();
    std::os::unix::fs::symlink(home.join("external.yaml"), cache.join("link.yaml")).unwrap();

    let output = run(home, &["cache", "delete", "--ingredients"]);
    assert!(output.status.success(), "{}", text(&output));
    for file in [
        "demo.yaml",
        "demo.json",
        "orphan.yaml",
        "broken.json",
        "link.yaml",
    ] {
        assert!(fs::symlink_metadata(cache.join(file)).is_err(), "{file}");
    }
    assert_eq!(
        fs::read_to_string(cache.join(".catalog.json")).unwrap(),
        "catalog contents"
    );
    assert_eq!(
        fs::read_to_string(cache.join("keep.txt")).unwrap(),
        "unrelated"
    );
    assert_eq!(
        fs::read_to_string(home.join("external.yaml")).unwrap(),
        "keep target"
    );
    assert!(
        run(home, &["cache", "delete", "--ingredients"])
            .status
            .success()
    );
}

#[test]
fn cache_delete_usage_lists_all_required_choices() {
    let dir = TempDir::new().unwrap();
    for (args, code) in [
        (&["cache", "delete"][..], 2),
        (&["cache", "delete", "--help"][..], 0),
    ] {
        let output = run(dir.path(), args);
        assert_eq!(output.status.code(), Some(code));
        assert!(
            text(&output).contains("Usage: cockup cache delete <NAMES|--ingredients|--catalog>"),
            "{}",
            text(&output)
        );
    }
}

#[test]
fn ingredient_commands_do_not_expose_cache_management() {
    let dir = TempDir::new().unwrap();
    for command in ["update", "status", "delete", "clean"] {
        let output = run(dir.path(), &["ingredient", command]);
        assert_eq!(output.status.code(), Some(2), "{}", text(&output));
    }
    let old_flag = run(dir.path(), &["ingredient", "search", "ghost", "--refresh"]);
    assert_eq!(old_flag.status.code(), Some(2), "{}", text(&old_flag));
    for args in [
        &["cache", "delete"][..],
        &["cache", "delete", "demo", "--catalog"][..],
        &["cache", "delete", "demo", "--ingredients"][..],
        &["cache", "delete", "--catalog", "--ingredients"][..],
        &["cache", "refresh", "demo", "--catalog"][..],
        &["cache", "refresh", "demo", "--ingredients"][..],
        &["cache", "refresh", "--ingredients", "--catalog"][..],
    ] {
        let output = run(dir.path(), args);
        assert_eq!(output.status.code(), Some(2), "{}", text(&output));
    }
}

#[test]
fn cache_refresh_scopes_network_checks() {
    for (args, expected) in [
        (
            &["cache", "refresh", "--ingredients"][..],
            vec!["/ingredient"],
        ),
        (&["cache", "refresh", "--catalog"][..], vec!["/catalog"]),
        (&["cache", "refresh"][..], vec!["/ingredient", "/catalog"]),
    ] {
        let dir = TempDir::new().unwrap();
        let home = dir.path();
        let cache = home.join("Library/Caches/cockup/ingredients/v1");
        fs::create_dir_all(&cache).unwrap();
        fs::write(
            cache.join("demo.yaml"),
            "rules:\n  - from: ~/config\n    targets: [file]\n    to: files\n",
        )
        .unwrap();
        fs::write(cache.join("demo.json"), br#"{"downloaded_at_unix":0,"checked_at_unix":0,"source_ref":"main","source":"","etag":"demo-v1"}"#).unwrap();
        fs::write(cache.join(".catalog.json"), br#"{"version":1,"source":"https://api.github.com/repos/huaium/cockup/contents/ingredients/library?ref=main","downloaded_at_unix":0,"checked_at_unix":0,"etag":"catalog-v1","names":["demo"]}"#).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let count = expected.len();
        let server = std::thread::spawn(move || {
            let mut paths = Vec::new();
            let start = Instant::now();
            while paths.len() < count && start.elapsed() < Duration::from_secs(3) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let mut request = [0; 4096];
                        let size = stream.read(&mut request).unwrap();
                        let line = String::from_utf8_lossy(&request[..size]);
                        paths.push(line.split_whitespace().nth(1).unwrap().to_string());
                        stream.write_all(b"HTTP/1.1 304 Not Modified\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10))
                    }
                    Err(error) => panic!("{error}"),
                }
            }
            paths
        });
        let output = Command::new(env!("CARGO_BIN_EXE_cockup"))
            .current_dir(home)
            .env("HOME", home)
            .env("NO_COLOR", "1")
            .env("COCKUP_INGREDIENT_TEST_URL", format!("{url}/ingredient"))
            .env(
                "COCKUP_INGREDIENT_CATALOG_TEST_URL",
                format!("{url}/catalog"),
            )
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", text(&output));
        let mut actual = server.join().unwrap();
        actual.sort();
        let mut expected = expected;
        expected.sort();
        assert_eq!(actual, expected);
    }
}
