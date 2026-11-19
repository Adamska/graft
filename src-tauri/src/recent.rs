use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::Result;

const FILE_NAME: &str = "recent.json";
const LIMIT: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentRepository {
    pub path: String,
    pub name: String,
    pub opened_at: u64,
}

/// Most recently opened repositories that still exist on disk, newest first.
pub fn list(config_dir: &Path) -> Vec<RecentRepository> {
    let stored = std::fs::read(config_dir.join(FILE_NAME)).ok();
    let entries: Vec<RecentRepository> =
        stored.and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
    entries.into_iter().filter(|entry| PathBuf::from(&entry.path).exists()).collect()
}

pub fn remember(config_dir: &Path, path: &str, name: &str) -> Result<()> {
    let mut entries = list(config_dir);
    entries.retain(|entry| entry.path != path);
    let opened_at = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs());
    entries.insert(0, RecentRepository { path: path.to_string(), name: name.to_string(), opened_at });
    entries.truncate(LIMIT);

    std::fs::create_dir_all(config_dir)?;
    let json = serde_json::to_vec_pretty(&entries).map_err(std::io::Error::other)?;
    Ok(std::fs::write(config_dir.join(FILE_NAME), json)?)
}
