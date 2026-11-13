use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_THUMB_CACHE_LIMIT_MB: u64 = 2048;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub last_workspace: Option<String>,
    #[serde(default = "default_thumb_cache_limit_mb")]
    pub thumb_cache_limit_mb: u64,
    #[serde(default = "default_watch_workspace")]
    pub watch_workspace: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            last_workspace: None,
            thumb_cache_limit_mb: DEFAULT_THUMB_CACHE_LIMIT_MB,
            watch_workspace: true,
        }
    }
}

fn default_thumb_cache_limit_mb() -> u64 {
    DEFAULT_THUMB_CACHE_LIMIT_MB
}

fn default_watch_workspace() -> bool {
    true
}

pub fn load(path: &Path) -> Settings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, settings: &Settings) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("创建设置目录失败: {}", parent.display()))?;
    }
    let temp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(settings)?;
    std::fs::write(&temp, bytes).with_context(|| format!("写入设置失败: {}", temp.display()))?;
    if path.exists() {
        std::fs::remove_file(path).with_context(|| format!("替换设置失败: {}", path.display()))?;
    }
    std::fs::rename(&temp, path).with_context(|| format!("保存设置失败: {}", path.display()))?;
    Ok(())
}
