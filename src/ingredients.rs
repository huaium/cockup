use crate::{config::Symlinks, report};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Component, Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const MAX_SIZE: usize = 256 * 1024;
const REFRESH_AFTER: Duration = Duration::from_secs(7 * 24 * 60 * 60);

pub enum Mode {
    Update,
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
    etag: Option<String>,
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

fn cache_dir() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
    Ok(PathBuf::from(home).join("Library/Caches/cockup/ingredients/v1"))
}

fn read_cache(name: &str, dir: &Path) -> Option<(String, CacheMetadata)> {
    let yaml = fs::read_to_string(dir.join(format!("{name}.yaml"))).ok()?;
    let metadata: CacheMetadata =
        serde_json::from_slice(&fs::read(dir.join(format!("{name}.json"))).ok()?).ok()?;
    if metadata.source_ref != "main" {
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
    let json = serde_json::to_vec_pretty(metadata).map_err(|error| error.to_string())?;
    atomic_write(&dir.join(format!("{name}.json")), &json)
}

struct Response {
    status: u16,
    etag: Option<String>,
    body: Vec<u8>,
}

fn fetch(name: &str, etag: Option<&str>) -> Result<Response, String> {
    let github_url = format!(
        "https://api.github.com/repos/huaium/cockup/contents/ingredients/library/{name}.yaml?ref=main"
    );
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
            Mode::Update | Mode::ReadOnly => {
                let cached = read_cache(name, &dir);
                let time = now();
                if let Some((yaml, metadata)) = &cached
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
                            if matches!(self.mode, Mode::Update) {
                                save_cache(
                                    name,
                                    &dir,
                                    &yaml,
                                    &CacheMetadata {
                                        downloaded_at_unix: time,
                                        checked_at_unix: time,
                                        source_ref: "main".to_string(),
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
                            if matches!(self.mode, Mode::Update) {
                                save_cache(name, &dir, &yaml, &metadata)?;
                            }
                            yaml
                        }
                        Ok(Response { status: 404, .. }) => {
                            return Err(format!(
                                "Ingredient `{name}` was not found in huaium/cockup"
                            ));
                        }
                        Ok(response) => {
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
