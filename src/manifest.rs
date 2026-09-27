use crate::config::Symlinks;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

pub const NAME: &str = ".cockup-symlinks.json";
#[derive(Clone, Deserialize, Serialize)]
pub struct TargetLink {
    pub location: PathBuf,
    pub target: PathBuf,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Link {
    pub location: PathBuf,
    pub target: PathBuf,
    pub resolved_target: PathBuf,
    pub mode: Symlinks,
    pub backup_path: PathBuf,
    pub target_chain: Vec<TargetLink>,
}
#[derive(Deserialize, Serialize)]
pub struct Manifest {
    pub version: u32,
    pub backup_user: String,
    pub backup_home: PathBuf,
    pub canonical_home: PathBuf,
    pub links: Vec<Link>,
}
impl Manifest {
    pub fn new() -> Result<Self, String> {
        let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
        Ok(Self {
            version: 1,
            backup_user: std::env::var("USER").unwrap_or_default(),
            canonical_home: fs::canonicalize(&home).unwrap_or_else(|_| PathBuf::from(&home)),
            backup_home: PathBuf::from(home),
            links: Vec::new(),
        })
    }
    pub fn load(root: &Path) -> Result<Option<Self>, String> {
        let path = root.join(NAME);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        let manifest: Self =
            serde_json::from_slice(&bytes).map_err(|e| format!("Invalid symlink manifest: {e}"))?;
        if manifest.version != 1
            || (!manifest.backup_home.is_absolute() || !manifest.canonical_home.is_absolute())
        {
            return Err("Unsupported or invalid symlink manifest".into());
        }
        let mut seen = std::collections::HashSet::new();
        for link in &manifest.links {
            if !link.location.is_absolute()
                || !link.resolved_target.is_absolute()
                || link.target_chain.iter().any(|l| !l.location.is_absolute())
                || link.mode == Symlinks::Prompt
                || link.backup_path.as_os_str().is_empty()
                || link
                    .backup_path
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_)))
                || link.backup_path == Path::new(NAME)
                || !seen.insert(&link.backup_path)
            {
                return Err("Invalid symlink manifest entry".into());
            }
        }
        Ok(Some(manifest))
    }
    pub fn save(&mut self, root: &Path) -> Result<(), String> {
        self.links.sort_by(|a, b| a.backup_path.cmp(&b.backup_path));
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let tmp = root.join(format!("{NAME}.{}.tmp", std::process::id()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|e| format!("Cannot create manifest temporary file: {e}"))?;
        let result = (|| -> std::io::Result<()> {
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&tmp, root.join(NAME))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result.map_err(|e| format!("Cannot save symlink manifest: {e}"))
    }
}
