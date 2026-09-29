use super::{atomic_write, cache_dir, now, read_cache, validate_name};
use crate::report;
use serde::{Deserialize, Serialize};
use std::{fs, io::Read, os::unix::fs::PermissionsExt, path::Path, time::Duration};

const SOURCE: &str =
    "https://api.github.com/repos/huaium/cockup/contents/ingredients/library?ref=main";
const FILE: &str = ".catalog.json";
const MAX_RESPONSE: usize = 2 * 1024 * 1024;
const REFRESH_AFTER: u64 = 24 * 60 * 60;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    version: u8,
    source: String,
    downloaded_at_unix: u64,
    checked_at_unix: u64,
    etag: Option<String>,
    names: Vec<String>,
}

#[derive(Deserialize)]
struct Entry {
    name: String,
    path: String,
    #[serde(rename = "type")]
    kind: String,
}

fn cached(dir: &Path) -> Option<Catalog> {
    let bytes = fs::read(dir.join(FILE)).ok()?;
    if bytes.len() > MAX_RESPONSE {
        return None;
    }
    let catalog: Catalog = serde_json::from_slice(&bytes).ok()?;
    if catalog.version != 1 || catalog.source != SOURCE || catalog.names.len() >= 1000 {
        return None;
    }
    if catalog
        .names
        .iter()
        .any(|name| validate_name(name).is_err())
        || catalog.names.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return None;
    }
    Some(catalog)
}

fn save(dir: &Path, catalog: &Catalog) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
        .map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec_pretty(catalog).map_err(|error| error.to_string())?;
    atomic_write(&dir.join(FILE), &bytes)
}

fn fetch(etag: Option<&str>) -> Result<(u16, Option<String>, Vec<u8>), String> {
    #[cfg(debug_assertions)]
    let test_url = std::env::var("COCKUP_INGREDIENT_CATALOG_TEST_URL").ok();
    #[cfg(not(debug_assertions))]
    let test_url: Option<String> = None;
    let local_test = test_url
        .as_deref()
        .is_some_and(|url| url.starts_with("http://127.0.0.1:"));
    if test_url.is_some() && !local_test {
        return Err("Ingredient catalog test URL must use loopback HTTP".into());
    }
    let url = test_url.as_deref().unwrap_or(SOURCE);
    let config = ureq::Agent::config_builder()
        .https_only(!local_test)
        .timeout_global(Some(Duration::from_secs(15)))
        .http_status_as_error(false)
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let mut request = agent
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "cockup");
    if let Some(etag) = etag {
        request = request.header("If-None-Match", etag);
    }
    let mut response = request.call().map_err(|error| error.to_string())?;
    let status = response.status().as_u16();
    let etag = response
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let mut body = Vec::new();
    if status == 200 {
        response
            .body_mut()
            .as_reader()
            .take((MAX_RESPONSE + 1) as u64)
            .read_to_end(&mut body)
            .map_err(|error| error.to_string())?;
        if body.len() > MAX_RESPONSE {
            return Err("Ingredient catalog response is too large".into());
        }
    }
    Ok((status, etag, body))
}

fn names(body: &[u8]) -> Result<Vec<String>, String> {
    let entries: Vec<Entry> = serde_json::from_slice(body)
        .map_err(|error| format!("Invalid GitHub ingredient catalog: {error}"))?;
    if entries.len() >= 1000 {
        return Err("GitHub ingredient directory reached its 1,000-file listing limit".into());
    }
    let mut names = Vec::new();
    for entry in entries {
        if entry.kind != "file" || !entry.name.ends_with(".yaml") {
            continue;
        }
        if entry.path != format!("ingredients/library/{}", entry.name) {
            return Err("GitHub ingredient catalog contains an unexpected path".into());
        }
        let name = entry.name.trim_end_matches(".yaml");
        validate_name(name)?;
        names.push(name.to_string());
    }
    names.sort();
    names.dedup();
    Ok(names)
}

fn catalog(dir: &Path, refresh: bool) -> Result<Catalog, String> {
    let previous = cached(dir);
    let current_time = now();
    if !refresh
        && let Some(catalog) = &previous
        && catalog.checked_at_unix <= current_time
        && current_time - catalog.checked_at_unix < REFRESH_AFTER
    {
        return Ok(catalog.clone());
    }
    let response = fetch(
        previous
            .as_ref()
            .and_then(|catalog| catalog.etag.as_deref()),
    );
    match response {
        Ok((200, etag, body)) => {
            let catalog = Catalog {
                version: 1,
                source: SOURCE.into(),
                downloaded_at_unix: current_time,
                checked_at_unix: current_time,
                etag,
                names: names(&body)?,
            };
            save(dir, &catalog)?;
            Ok(catalog)
        }
        Ok((304, _, _)) => {
            let mut catalog = previous.ok_or("GitHub returned 304 without a cached catalog")?;
            catalog.checked_at_unix = current_time;
            save(dir, &catalog)?;
            Ok(catalog)
        }
        Ok((404, _, _)) => Err("GitHub ingredient library was not found".into()),
        Ok((status, _, _)) => fallback(previous, &format!("GitHub returned HTTP {status}")),
        Err(error) => fallback(previous, &error),
    }
}

fn fallback(previous: Option<Catalog>, reason: &str) -> Result<Catalog, String> {
    if let Some(catalog) = previous {
        report::warning(&format!(
            "Could not refresh ingredient catalog ({reason}); using cached list"
        ));
        Ok(catalog)
    } else {
        Err(format!("Cannot fetch ingredient catalog: {reason}"))
    }
}

pub(crate) fn search(query: &str, refresh: bool) -> Result<(), String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Search query must not be empty".into());
    }
    let normalized = query
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .to_ascii_lowercase();
    let dir = cache_dir()?;
    let catalog = catalog(&dir, refresh)?;
    let matches: Vec<_> = catalog
        .names
        .iter()
        .filter(|name| name.contains(&normalized))
        .collect();
    if matches.is_empty() {
        println!("No ingredients match `{query}`.");
    } else {
        for name in matches {
            if read_cache(name, &dir).is_some() {
                println!("{name} [cached]");
            } else {
                println!("{name}");
            }
        }
    }
    Ok(())
}
