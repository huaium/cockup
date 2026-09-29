use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    time::{Duration, Instant},
};

fn request(root: &Path, args: &[&str], status: u16, body: &[u8]) -> (Output, String) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/catalog", listener.local_addr().unwrap());
    let body = body.to_vec();
    let server = std::thread::spawn(move || {
        let start = Instant::now();
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(pair) => break pair,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && start.elapsed() < Duration::from_secs(2) =>
                {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(error) => panic!("No catalog request: {error}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = [0; 2048];
        let count = stream.read(&mut request).unwrap();
        write!(stream, "HTTP/1.1 {status} Test\r\nETag: \"catalog-1\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
        stream.write_all(&body).unwrap();
        String::from_utf8_lossy(&request[..count]).to_string()
    });
    let output = Command::new(env!("CARGO_BIN_EXE_cockup"))
        .current_dir(root)
        .env("HOME", root)
        .env("NO_COLOR", "1")
        .env("COCKUP_INGREDIENT_CATALOG_TEST_URL", url)
        .args(args)
        .output()
        .unwrap();
    let request = server.join().unwrap();
    (output, request)
}

#[test]
fn ingredient_search_lists_matching_remote_names_without_downloading_recipes() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let body = br#"[{"name":"visual-studio-code.yaml","path":"ingredients/library/visual-studio-code.yaml","type":"file"},{"name":"ghostty.yaml","path":"ingredients/library/ghostty.yaml","type":"file"},{"name":"visual-studio.yaml","path":"ingredients/library/visual-studio.yaml","type":"file"},{"name":"README.md","path":"ingredients/library/README.md","type":"file"}]"#;
    let (output, sent) = request(root, &["ingredient", "search", "VISUAL studio"], 200, body);
    assert!(sent.contains("GET /catalog"));
    assert!(output.status.success(), "{}", text(&output));
    let result = text(&output);
    assert!(result.contains("visual-studio\n"), "{result}");
    assert!(result.contains("visual-studio-code\n"), "{result}");
    assert!(result.find("visual-studio\n").unwrap() < result.find("visual-studio-code\n").unwrap());
    assert!(!result.contains("ghostty"));
    assert!(
        !root
            .join("Library/Caches/cockup/ingredients/v1/visual-studio.yaml")
            .exists()
    );
    let cache_dir = root.join("Library/Caches/cockup/ingredients/v1");
    fs::write(
        cache_dir.join("visual-studio.yaml"),
        "rules:\n  - from: ~/config\n    targets: [file]\n    to: files\n",
    )
    .unwrap();
    fs::write(cache_dir.join("visual-studio.json"), br#"{"downloaded_at_unix":0,"checked_at_unix":0,"source_ref":"main","source":"https://api.github.com/repos/huaium/cockup/contents/ingredients/library/visual-studio.yaml?ref=main","etag":null}"#).unwrap();
    let cached = Command::new(env!("CARGO_BIN_EXE_cockup"))
        .current_dir(root)
        .env("HOME", root)
        .env("NO_COLOR", "1")
        .env(
            "COCKUP_INGREDIENT_CATALOG_TEST_URL",
            "http://127.0.0.1:1/catalog",
        )
        .args(["ingredient", "search", "visual-studio"])
        .output()
        .unwrap();
    assert!(cached.status.success(), "{}", text(&cached));
    assert!(text(&cached).contains("visual-studio [cached]"));
    let missing = Command::new(env!("CARGO_BIN_EXE_cockup"))
        .current_dir(root)
        .env("HOME", root)
        .env("NO_COLOR", "1")
        .args(["ingredient", "search", "terminal"])
        .output()
        .unwrap();
    assert!(missing.status.success(), "{}", text(&missing));
    assert!(text(&missing).contains("No ingredients match"));
}

#[test]
fn ingredient_search_reuses_cache_refreshes_with_etag_and_falls_back_offline() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let body =
        br#"[{"name":"ghostty.yaml","path":"ingredients/library/ghostty.yaml","type":"file"}]"#;
    let (first, _) = request(root, &["ingredient", "search", "ghost"], 200, body);
    assert!(first.status.success(), "{}", text(&first));
    let cache = root.join("Library/Caches/cockup/ingredients/v1/.catalog.json");
    assert!(cache.exists());

    let offline = || {
        Command::new(env!("CARGO_BIN_EXE_cockup"))
            .current_dir(root)
            .env("HOME", root)
            .env("NO_COLOR", "1")
            .env(
                "COCKUP_INGREDIENT_CATALOG_TEST_URL",
                "http://127.0.0.1:1/catalog",
            )
            .args(["ingredient", "search", "ghost"])
            .output()
            .unwrap()
    };
    let fresh = offline();
    assert!(fresh.status.success(), "{}", text(&fresh));
    assert!(!text(&fresh).contains("using cached list"));

    let (refreshed, sent) = request(
        root,
        &["ingredient", "search", "ghost", "--refresh"],
        304,
        b"",
    );
    assert!(refreshed.status.success(), "{}", text(&refreshed));
    assert!(
        sent.to_ascii_lowercase()
            .contains("if-none-match: \"catalog-1\""),
        "{sent}"
    );
    let mut catalog: serde_json::Value =
        serde_json::from_slice(&fs::read(&cache).unwrap()).unwrap();
    catalog["checked_at_unix"] = 0.into();
    fs::write(&cache, serde_json::to_vec(&catalog).unwrap()).unwrap();
    let stale = offline();
    assert!(stale.status.success(), "{}", text(&stale));
    assert!(text(&stale).contains("using cached list"));
}

#[test]
fn ingredient_search_fails_when_github_is_unavailable_without_catalog_cache() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let result = Command::new(env!("CARGO_BIN_EXE_cockup"))
        .current_dir(root)
        .env("HOME", root)
        .env(
            "COCKUP_INGREDIENT_CATALOG_TEST_URL",
            "http://127.0.0.1:1/catalog",
        )
        .args(["ingredient", "search", "ghostty"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1), "{}", text(&result));
    assert!(text(&result).contains("Cannot fetch ingredient catalog"));
}
