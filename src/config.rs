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
    #[serde(default, deserialize_with = "null_default")]
    pub targets: Vec<String>,
    #[serde(default, rename = "on-start", deserialize_with = "null_default")]
    pub on_start: Vec<Hook>,
    #[serde(default, rename = "on-end", deserialize_with = "null_default")]
    pub on_end: Vec<Hook>,
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
#[derive(Deserialize)]
pub struct Config {
    pub destination: PathBuf,
    pub rules: Vec<Rule>,
    #[serde(default, deserialize_with = "null_default")]
    pub hooks: Hooks,
    #[serde(default)]
    pub clean: bool,
    #[serde(default = "default_true")]
    pub metadata: bool,
    #[serde(default, deserialize_with = "null_default")]
    include: Vec<PathBuf>,
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
    fn read(path: &Path, stack: &mut HashSet<PathBuf>) -> Result<Config> {
        let path = fs::canonicalize(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if !stack.insert(path.clone()) {
            return Err(format!("Include cycle at {}", path.display()));
        }
        let source = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let mut cfg: Config =
            serde_yaml_ng::from_str(&source).map_err(|e| format!("{}: {e}", path.display()))?;
        cfg.directory = path.parent().unwrap().to_path_buf();
        cfg.destination = absolute(&cfg.destination, &cfg.directory)?;
        for rule in &mut cfg.rules {
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
            let child = read(&absolute(include, &cfg.directory)?, stack)?;
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
    read(path, &mut HashSet::new())
}
