use crate::ingredients::{Mode, Resolver};
use serde::{Deserialize, Deserializer};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, String>;
fn default_true() -> bool {
    true
}
fn null_default<'de, D, T>(d: D) -> std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}
fn environment<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<BTreeMap<String, String>, D::Error> {
    let values =
        Option::<BTreeMap<String, serde_yaml_ng::Value>>::deserialize(d)?.unwrap_or_default();
    values
        .into_iter()
        .map(|(key, value)| {
            let text = match value {
                serde_yaml_ng::Value::String(s) => s,
                serde_yaml_ng::Value::Number(n) => n.to_string(),
                serde_yaml_ng::Value::Bool(b) => b.to_string(),
                _ => {
                    return Err(serde::de::Error::custom(
                        "hook env values must be strings, numbers, or booleans",
                    ));
                }
            };
            Ok((key, text))
        })
        .collect()
}
#[derive(Clone, Deserialize)]
pub struct Hook {
    pub name: String,
    #[serde(deserialize_with = "shell_command")]
    pub command: String,
    #[serde(default, deserialize_with = "shell_check")]
    pub check: Option<String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    #[serde(default)]
    pub output: bool,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default, deserialize_with = "environment")]
    pub env: BTreeMap<String, String>,
}
fn shell_text<E: serde::de::Error>(value: serde_yaml_ng::Value) -> std::result::Result<String, E> {
    match value {
        serde_yaml_ng::Value::String(text) if !text.trim().is_empty() => Ok(text),
        serde_yaml_ng::Value::Sequence(_) => Err(E::custom(
            "command/check arrays are no longer supported; use a shell string executed by /bin/sh -cu",
        )),
        _ => Err(E::custom("command/check must be a nonempty shell string")),
    }
}
fn shell_command<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<String, D::Error> {
    shell_text(serde_yaml_ng::Value::deserialize(d)?)
}
fn shell_check<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Option<String>, D::Error> {
    Option::<serde_yaml_ng::Value>::deserialize(d)?
        .map(shell_text)
        .transpose()
}
#[derive(Deserialize)]
pub struct Rule {
    #[serde(rename = "from")]
    pub src: PathBuf,
    pub to: PathBuf,
    #[serde(default)]
    pub symlinks: Option<Symlinks>,
    #[serde(default)]
    pub metadata: Option<bool>,
    #[serde(default, deserialize_with = "null_default")]
    pub targets: Vec<String>,
    #[serde(default, rename = "on-start", deserialize_with = "null_default")]
    pub on_start: Vec<Hook>,
    #[serde(default, rename = "on-end", deserialize_with = "null_default")]
    pub on_end: Vec<Hook>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Include {
    #[serde(default)]
    file: Option<PathBuf>,
    #[serde(default)]
    ingredient: Option<String>,
    #[serde(default)]
    wrap: Option<PathBuf>,
    #[serde(default)]
    symlinks: Option<Symlinks>,
    #[serde(default)]
    metadata: Option<bool>,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Hooks {
    #[serde(default, deserialize_with = "null_default")]
    pub pre_backup: Vec<Hook>,
    #[serde(default, deserialize_with = "null_default")]
    pub post_backup: Vec<Hook>,
    #[serde(default, deserialize_with = "null_default")]
    pub pre_restore: Vec<Hook>,
    #[serde(default, deserialize_with = "null_default")]
    pub post_restore: Vec<Hook>,
}
impl Hooks {
    fn all_mut(&mut self) -> impl Iterator<Item = &mut Hook> {
        self.pre_backup
            .iter_mut()
            .chain(&mut self.post_backup)
            .chain(&mut self.pre_restore)
            .chain(&mut self.post_restore)
    }
    fn append(&mut self, other: Self) {
        self.pre_backup.extend(other.pre_backup);
        self.post_backup.extend(other.post_backup);
        self.pre_restore.extend(other.pre_restore);
        self.post_restore.extend(other.post_restore);
    }
    pub fn all(&self) -> impl Iterator<Item = &Hook> {
        self.pre_backup
            .iter()
            .chain(&self.post_backup)
            .chain(&self.pre_restore)
            .chain(&self.post_restore)
    }
}
#[derive(Clone, Copy, Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Symlinks {
    Reference,
    Dereference,
    Prompt,
}
#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub symlinks: Option<Symlinks>,
    #[serde(default, deserialize_with = "null_default")]
    pub destination: PathBuf,
    pub rules: Vec<Rule>,
    #[serde(default, deserialize_with = "null_default")]
    pub hooks: Hooks,
    #[serde(default)]
    pub clean: bool,
    #[serde(default = "default_true")]
    pub metadata: bool,
    #[serde(default, deserialize_with = "null_default")]
    include: Vec<Include>,
    #[serde(skip)]
    pub directory: PathBuf,
    #[serde(skip)]
    pub ingredients: BTreeMap<String, String>,
}
impl Config {
    pub fn all_hooks(&self) -> impl Iterator<Item = &Hook> {
        self.rules
            .iter()
            .flat_map(|r| r.on_start.iter().chain(&r.on_end))
            .chain(self.hooks.all())
    }
}
pub fn absolute(path: &Path, base: &Path) -> Result<PathBuf> {
    let path = if path == Path::new("~") || path.starts_with("~/") {
        let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
        PathBuf::from(home).join(path.strip_prefix("~").unwrap())
    } else {
        path.to_path_buf()
    };
    Ok(if path.is_absolute() {
        path
    } else {
        base.join(path)
    })
}
pub fn destination(path: &Path) -> Result<PathBuf> {
    let path = fs::canonicalize(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let source = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let cfg: Config =
        serde_yaml_ng::from_str(&source).map_err(|error| format!("{}: {error}", path.display()))?;
    if cfg.destination.as_os_str().is_empty() {
        return Err(format!("{}: destination is required", path.display()));
    }
    absolute(&cfg.destination, path.parent().unwrap())
}

pub fn load(path: &Path, mode: Mode) -> Result<Config> {
    fn read(
        path: &Path,
        override_source: Option<&str>,
        stack: &mut HashSet<PathBuf>,
        inherited: Option<(Symlinks, bool)>,
        wrap: &Path,
        resolver: &mut Resolver,
    ) -> Result<Config> {
        let path = if override_source.is_some() {
            path.to_path_buf()
        } else {
            fs::canonicalize(path).map_err(|e| format!("{}: {e}", path.display()))?
        };
        if !stack.insert(path.clone()) {
            return Err(format!("Include cycle at {}", path.display()));
        }
        let source = match override_source {
            Some(source) => source.to_string(),
            None => fs::read_to_string(&path).map_err(|e| e.to_string())?,
        };
        let mut cfg: Config =
            serde_yaml_ng::from_str(&source).map_err(|e| format!("{}: {e}", path.display()))?;
        cfg.directory = path.parent().unwrap().to_path_buf();
        if inherited.is_none() && cfg.destination.as_os_str().is_empty() {
            return Err(format!(
                "{}: destination is required to run this config\n=> configs without it are include-only ingredients",
                path.display()
            ));
        }
        if !cfg.destination.as_os_str().is_empty() {
            cfg.destination = absolute(&cfg.destination, &cfg.directory)?;
        }
        let (symlinks, metadata) = match inherited {
            Some(settings) => settings,
            None => (
                cfg.symlinks
                    .ok_or_else(|| format!("{}: symlinks is required", path.display()))?,
                cfg.metadata,
            ),
        };
        for (index, rule) in cfg.rules.iter_mut().enumerate() {
            for (field, pattern) in std::iter::once(("from", rule.src.to_string_lossy())).chain(
                rule.targets
                    .iter()
                    .map(|target| ("targets", target.as_str().into())),
            ) {
                glob::Pattern::new(&pattern).map_err(|error| {
                    format!(
                        "{}: rule {}: invalid {field} glob {pattern:?}: {error}",
                        path.display(),
                        index + 1
                    )
                })?;
            }
            rule.src = absolute(&rule.src, &cfg.directory)?;
            for relative in
                std::iter::once(rule.to.as_path()).chain(rule.targets.iter().map(Path::new))
            {
                if relative.is_absolute()
                    || relative
                        .components()
                        .any(|c| matches!(c, std::path::Component::ParentDir))
                {
                    return Err("Rule targets and to must be relative paths without '..'".into());
                }
            }
            rule.to = wrap.join(&rule.to);
            rule.symlinks = Some(rule.symlinks.unwrap_or(symlinks));
            rule.metadata = Some(rule.metadata.unwrap_or(metadata));
        }
        for h in cfg
            .rules
            .iter_mut()
            .flat_map(|r| r.on_start.iter_mut().chain(&mut r.on_end))
            .chain(cfg.hooks.all_mut())
        {
            if let Some(cwd) = &h.cwd {
                if cwd.as_os_str().is_empty() {
                    return Err(format!("Hook `{}` cwd must not be empty", h.name));
                }
                h.cwd = Some(absolute(cwd, &cfg.directory)?);
            }
            if h.name.is_empty() {
                return Err("Hook name must not be empty".into());
            }
        }
        let mut rules = Vec::new();
        let mut hooks = Hooks::default();
        for include in &cfg.include {
            if include.file.is_some() == include.ingredient.is_some() {
                return Err(format!(
                    "{}: include must specify exactly one of file or ingredient",
                    path.display()
                ));
            }
            let prefix = if let Some(folder) = &include.wrap {
                if folder.as_os_str().is_empty()
                    || folder.is_absolute()
                    || folder
                        .components()
                        .any(|c| matches!(c, std::path::Component::ParentDir))
                    || folder == Path::new(".")
                {
                    return Err(format!(
                        "{}: include wrap must be a non-empty relative path without '..'",
                        path.display()
                    ));
                }
                wrap.join(folder)
            } else {
                wrap.to_path_buf()
            };
            let (child_path, child_source) = if let Some(file) = &include.file {
                (absolute(file, &cfg.directory)?, None)
            } else {
                let (path, source) = resolver.resolve(include.ingredient.as_deref().unwrap())?;
                (path, Some(source))
            };
            let child = read(
                &child_path,
                child_source.as_deref(),
                stack,
                Some((
                    include.symlinks.unwrap_or(symlinks),
                    include.metadata.unwrap_or(metadata),
                )),
                &prefix,
                resolver,
            )?;
            rules.extend(child.rules);
            hooks.append(child.hooks);
        }
        rules.extend(cfg.rules);
        hooks.append(cfg.hooks);
        cfg.rules = rules;
        cfg.hooks = hooks;
        stack.remove(&path);
        Ok(cfg)
    }
    let mut resolver = Resolver::new(mode);
    let mut cfg = read(
        path,
        None,
        &mut HashSet::new(),
        None,
        Path::new(""),
        &mut resolver,
    )?;
    cfg.ingredients = resolver.used;
    Ok(cfg)
}
