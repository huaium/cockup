use crate::{config::Symlinks, report};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Component, Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const MAX_SIZE: usize = 256 * 1024;
const REFRESH_AFTER: Duration = Duration::from_secs(30 * 24 * 60 * 60);

pub enum Mode {
    Update,
    ForceUpdate,
    ReadOnly,
    Snapshot(BTreeMap<String, String>),
}

pub struct Resolver {
    mode: Mode,
    pub used: BTreeMap<String, String>,
}

#[derive(Clone, Deserialize, Serialize)]
struct CacheMetadata {
    downloaded_at_unix: u64,
    checked_at_unix: u64,
    source_ref: String,
    #[serde(default)]
    source: String,
    etag: Option<String>,
}

fn source_url(name: &str) -> String {
    format!(
        "https://api.github.com/repos/huaium/cockup/contents/ingredients/library/{name}.yaml?ref=main"
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RemoteIngredient {
    rules: Vec<RemoteRule>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RemoteRule {
    #[serde(rename = "from")]
    source: PathBuf,
    targets: Vec<String>,
    to: PathBuf,
    #[serde(default, rename = "symlinks")]
    _symlinks: Option<Symlinks>,
    #[serde(default, rename = "metadata")]
    _metadata: Option<bool>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.starts_with('-')
        || name.ends_with('-')
        || !name
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return Err(format!("Invalid ingredient name: {name}"));
    }
    Ok(())
}

fn validate_yaml(name: &str, source: &str) -> Result<(), String> {
    if source.len() > MAX_SIZE {
        return Err(format!("Ingredient `{name}` exceeds {MAX_SIZE} bytes"));
    }
    let ingredient: RemoteIngredient = serde_yaml_ng::from_str(source)
        .map_err(|error| format!("Invalid ingredient `{name}`: {error}"))?;
    if ingredient.rules.is_empty() {
        return Err(format!("Ingredient `{name}` has no rules"));
    }
    for rule in &ingredient.rules {
        if !(rule.source == Path::new("~") || rule.source.starts_with("~/"))
            || rule
                .source
                .components()
                .any(|part| part == Component::ParentDir)
        {
            return Err(format!("Ingredient `{name}` sources must start with ~/"));
        }
        if rule.targets.is_empty() || rule.to.as_os_str().is_empty() {
            return Err(format!("Ingredient `{name}` has an incomplete rule"));
        }
        for relative in std::iter::once(rule.to.as_path()).chain(rule.targets.iter().map(Path::new))
        {
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|part| part == Component::ParentDir)
            {
                return Err(format!(
                    "Ingredient `{name}` paths must be relative without '..'"
                ));
            }
        }
        for pattern in std::iter::once(rule.source.to_string_lossy())
            .chain(rule.targets.iter().map(|target| target.as_str().into()))
        {
            glob::Pattern::new(&pattern)
                .map_err(|error| format!("Invalid ingredient `{name}` glob: {error}"))?;
        }
    }
    Ok(())
}

fn cache_root() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
    Ok(PathBuf::from(home).join("Library/Caches/cockup"))
}

fn cache_dir() -> Result<PathBuf, String> {
    Ok(cache_root()?.join("ingredients/v1"))
}

fn read_cache(name: &str, dir: &Path) -> Option<(String, CacheMetadata)> {
    let yaml = fs::read_to_string(dir.join(format!("{name}.yaml"))).ok()?;
    let metadata: CacheMetadata =
        serde_json::from_slice(&fs::read(dir.join(format!("{name}.json"))).ok()?).ok()?;
    if metadata.source_ref != "main"
        || (!metadata.source.is_empty() && metadata.source != source_url(name))
    {
        return None;
    }
    validate_yaml(name, &yaml).ok()?;
    Some((yaml, metadata))
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!("{}.{}.tmp", std::process::id(), now()));
    let result = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|error| format!("{}: {error}", path.display()))
}

fn save_cache(name: &str, dir: &Path, yaml: &str, metadata: &CacheMetadata) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
        .map_err(|error| error.to_string())?;
    atomic_write(&dir.join(format!("{name}.yaml")), yaml.as_bytes())?;
    save_metadata(name, dir, metadata)
}

fn save_metadata(name: &str, dir: &Path, metadata: &CacheMetadata) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(metadata).map_err(|error| error.to_string())?;
    atomic_write(&dir.join(format!("{name}.json")), &json)
}

struct Response {
    status: u16,
    etag: Option<String>,
    body: Vec<u8>,
}

fn fetch(name: &str, etag: Option<&str>) -> Result<Response, String> {
    let github_url = source_url(name);
    // Integration tests use a loopback HTTP server; release builds always use GitHub.
    #[cfg(debug_assertions)]
    let test_url = std::env::var("COCKUP_INGREDIENT_TEST_URL").ok();
    #[cfg(not(debug_assertions))]
    let test_url: Option<String> = None;
    let local_test = test_url
        .as_deref()
        .is_some_and(|url| url.starts_with("http://127.0.0.1:"));
    if test_url.is_some() && !local_test {
        return Err("Ingredient test URL must use loopback HTTP".into());
    }
    let url = test_url.unwrap_or(github_url);
    let config = ureq::Agent::config_builder()
        .https_only(!local_test)
        .timeout_global(Some(Duration::from_secs(15)))
        .http_status_as_error(false)
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let mut request = agent
        .get(&url)
        .header("Accept", "application/vnd.github.raw+json")
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
            .take((MAX_SIZE + 1) as u64)
            .read_to_end(&mut body)
            .map_err(|error| error.to_string())?;
        if body.len() > MAX_SIZE {
            return Err(format!("Ingredient `{name}` exceeds {MAX_SIZE} bytes"));
        }
    }
    Ok(Response { status, etag, body })
}

impl Resolver {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            used: BTreeMap::new(),
        }
    }

    pub fn resolve(&mut self, name: &str) -> Result<(PathBuf, String), String> {
        validate_name(name)?;
        let dir = cache_dir()?;
        let path = dir.join(format!("{name}.yaml"));
        let yaml = match &self.mode {
            Mode::Snapshot(snapshot) => snapshot
                .get(name)
                .cloned()
                .ok_or_else(|| format!("Missing backup snapshot for ingredient `{name}`"))?,
            Mode::Update | Mode::ForceUpdate | Mode::ReadOnly => {
                let cached = read_cache(name, &dir);
                let time = now();
                if !matches!(self.mode, Mode::ForceUpdate)
                    && let Some((yaml, metadata)) = &cached
                    && metadata.checked_at_unix <= time
                    && time - metadata.checked_at_unix < REFRESH_AFTER.as_secs()
                {
                    yaml.clone()
                } else {
                    let etag = cached
                        .as_ref()
                        .and_then(|(_, metadata)| metadata.etag.as_deref());
                    match fetch(name, etag) {
                        Ok(Response {
                            status: 200,
                            etag,
                            body,
                        }) => {
                            let yaml = String::from_utf8(body).map_err(|error| {
                                format!("Ingredient `{name}` is not UTF-8: {error}")
                            })?;
                            validate_yaml(name, &yaml)?;
                            if matches!(self.mode, Mode::Update | Mode::ForceUpdate) {
                                save_cache(
                                    name,
                                    &dir,
                                    &yaml,
                                    &CacheMetadata {
                                        downloaded_at_unix: time,
                                        checked_at_unix: time,
                                        source_ref: "main".to_string(),
                                        source: source_url(name),
                                        etag,
                                    },
                                )?;
                            }
                            yaml
                        }
                        Ok(Response { status: 304, .. }) => {
                            let (yaml, mut metadata) = cached.ok_or_else(|| {
                                format!("GitHub returned 304 for uncached ingredient `{name}`")
                            })?;
                            metadata.checked_at_unix = time;
                            if matches!(self.mode, Mode::Update | Mode::ForceUpdate) {
                                save_metadata(name, &dir, &metadata)?;
                            }
                            yaml
                        }
                        Ok(Response { status: 404, .. }) => {
                            return Err(format!(
                                "Ingredient `{name}` was not found in huaium/cockup"
                            ));
                        }
                        Ok(response) => {
                            if matches!(self.mode, Mode::ForceUpdate) {
                                return Err(format!(
                                    "Cannot update ingredient `{name}`: GitHub returned HTTP {}",
                                    response.status
                                ));
                            }
                            if let Some((yaml, _)) = cached {
                                report::warning(&format!(
                                    "Could not refresh ingredient `{name}` (HTTP {}); using cached copy",
                                    response.status
                                ));
                                yaml
                            } else {
                                return Err(format!(
                                    "Cannot fetch ingredient `{name}`: GitHub returned HTTP {}",
                                    response.status
                                ));
                            }
                        }
                        Err(error) => {
                            if matches!(self.mode, Mode::ForceUpdate) {
                                return Err(format!("Cannot update ingredient `{name}`: {error}"));
                            }
                            if let Some((yaml, _)) = cached {
                                report::warning(&format!(
                                    "Could not refresh ingredient `{name}` ({error}); using cached copy"
                                ));
                                yaml
                            } else {
                                return Err(format!("Cannot fetch ingredient `{name}`: {error}"));
                            }
                        }
                    }
                }
            }
        };
        validate_yaml(name, &yaml)?;
        self.used.insert(name.to_string(), yaml.clone());
        Ok((path, yaml))
    }
}

pub fn update(name: &str) -> Result<(), String> {
    validate_name(name)?;
    if read_cache(name, &cache_dir()?).is_none() {
        report::warning(&format!("Ingredient `{name}` is not cached."));
        if !report::confirm_with_prompt(&format!(
            "Download ingredient `{name}` from GitHub? [y/N]: "
        ))? {
            return Ok(());
        }
    }
    Resolver::new(Mode::ForceUpdate).resolve(name)?;
    println!("Ingredient `{name}` checked against GitHub.");
    status(Some(name))
}

fn utc_time(seconds: u64) -> String {
    if seconds > 253_402_300_799 {
        return format!("{seconds} Unix seconds");
    }
    let days = (seconds / 86_400) as i64;
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    let hour = seconds % 86_400 / 3_600;
    let minute = seconds % 3_600 / 60;
    let second = seconds % 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} UTC")
}

pub fn status(name: Option<&str>) -> Result<(), String> {
    let dir = cache_dir()?;
    let names = if let Some(name) = name {
        validate_name(name)?;
        vec![name.to_string()]
    } else if dir.exists() {
        let mut names = Vec::new();
        for entry in fs::read_dir(&dir).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "json")
                && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
                && validate_name(stem).is_ok()
            {
                names.push(stem.to_string());
            }
        }
        names.sort();
        names
    } else {
        Vec::new()
    };
    if names.is_empty() {
        println!("No cached ingredients.");
        return Ok(());
    }
    for name in names {
        let (_, metadata) = read_cache(&name, &dir)
            .ok_or_else(|| format!("Ingredient `{name}` has no valid cached YAML and metadata"))?;
        let source = if metadata.source.is_empty() {
            source_url(&name)
        } else {
            metadata.source
        };
        let time = now();
        let fresh = metadata.checked_at_unix <= time
            && time - metadata.checked_at_unix < REFRESH_AFTER.as_secs();
        println!("{name}:");
        println!("  Source: {source}");
        println!("  Downloaded: {}", utc_time(metadata.downloaded_at_unix));
        println!("  Checked: {}", utc_time(metadata.checked_at_unix));
        println!("  ETag: {}", metadata.etag.as_deref().unwrap_or("none"));
        println!("  Cache: {}", if fresh { "fresh" } else { "stale" });
    }
    Ok(())
}

fn checked_cache_dir() -> Result<Option<PathBuf>, String> {
    let dir = cache_dir()?;
    match fs::symlink_metadata(&dir) {
        Ok(metadata) if metadata.is_dir() => Ok(Some(dir)),
        Ok(_) => Err(format!(
            "Ingredient cache path is not a directory: {}",
            dir.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("{}: {error}", dir.display())),
    }
}

pub fn delete(names: &[String]) -> Result<(), String> {
    for name in names {
        validate_name(name)?;
    }
    let Some(dir) = checked_cache_dir()? else {
        return Err("No ingredients are cached".into());
    };
    let mut failed = 0;
    let mut seen = HashSet::new();
    for name in names {
        if !seen.insert(name) {
            continue;
        }
        match delete_one(&dir, name) {
            Ok(true) => println!("Deleted cached ingredient `{name}`."),
            Ok(false) => {
                report::error(&format!("Ingredient `{name}` is not cached"));
                failed += 1;
            }
            Err(error) => {
                report::error(&error);
                failed += 1;
            }
        }
    }
    if failed > 0 {
        Err(format!("Failed to delete {failed} ingredient(s)"))
    } else {
        Ok(())
    }
}

fn delete_one(dir: &Path, name: &str) -> Result<bool, String> {
    let mut removed = false;
    for extension in ["json", "yaml"] {
        let path = dir.join(format!("{name}.{extension}"));
        match fs::remove_file(&path) {
            Ok(()) => removed = true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("{}: {error}", path.display())),
        }
    }
    Ok(removed)
}

pub fn clean() -> Result<(), String> {
    let root = cache_root()?;
    match fs::symlink_metadata(&root) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => {
            return Err(format!(
                "Cockup cache path is not a directory: {}",
                root.display()
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            println!("No Cockup cache to clean.");
            return Ok(());
        }
        Err(error) => return Err(format!("{}: {error}", root.display())),
    }
    fs::remove_dir_all(&root).map_err(|error| format!("{}: {error}", root.display()))?;
    println!("Removed Cockup cache: {}", root.display());
    Ok(())
}
