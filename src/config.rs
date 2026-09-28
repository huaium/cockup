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
    pub command: Vec<String>,
    #[serde(default)]
    pub check: Option<Vec<String>>,
    #[serde(default)]
    pub output: bool,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default, deserialize_with = "environment")]
    pub env: BTreeMap<String, String>,
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
    file: PathBuf,
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
    Referece,
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
pub fn load(path: &Path) -> Result<Config> {
    fn read(
        path: &Path,
        stack: &mut HashSet<PathBuf>,
        inherited: Option<(Symlinks, bool)>,
        wrap: &Path,
    ) -> Result<Config> {
        let path = fs::canonicalize(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if !stack.insert(path.clone()) {
            return Err(format!("Include cycle at {}", path.display()));
        }
        let source = fs::read_to_string(&path).map_err(|e| e.to_string())?;
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
        for h in cfg.all_hooks() {
            if h.name.is_empty()
                || h.command.is_empty()
                || h.command[0].is_empty()
                || h.check
                    .as_ref()
                    .is_some_and(|c| c.is_empty() || c[0].is_empty())
            {
                return Err("Hook name and command/check executable must not be empty".into());
            }
        }
        let mut rules = Vec::new();
        let mut hooks = Hooks::default();
        for include in &cfg.include {
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
            let child = read(
                &absolute(&include.file, &cfg.directory)?,
                stack,
                Some((
                    include.symlinks.unwrap_or(symlinks),
                    include.metadata.unwrap_or(metadata),
                )),
                &prefix,
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
    read(path, &mut HashSet::new(), None, Path::new(""))
}
