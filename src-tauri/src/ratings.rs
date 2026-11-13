use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

const SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    version: u32,
    exported_at: i64,
    ratings: Vec<SnapshotEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotEntry {
    rel_path: String,
    stars: u8,
    updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RatingResult {
    pub id: i64,
    pub rel_path: String,
    pub stars: Option<u8>,
    pub updated_at: i64,
}

pub fn set(conn: &Connection, id: i64, stars: u8) -> Result<RatingResult> {
    if !(1..=5).contains(&stars) {
        bail!("评级必须为 1–5 星");
    }
    let rel_path = image_rel_path(conn, id)?;
    let updated_at = now_secs();
    conn.execute(
        "INSERT INTO image_rating(rel_path, stars, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(rel_path) DO UPDATE SET
           stars = excluded.stars,
           updated_at = excluded.updated_at",
        params![rel_path, stars, updated_at],
    )?;
    Ok(RatingResult {
        id,
        rel_path,
        stars: Some(stars),
        updated_at,
    })
}

pub fn clear(conn: &Connection, id: i64) -> Result<RatingResult> {
    let rel_path = image_rel_path(conn, id)?;
    conn.execute(
        "DELETE FROM image_rating WHERE rel_path = ?1",
        params![rel_path],
    )?;
    Ok(RatingResult {
        id,
        rel_path,
        stars: None,
        updated_at: now_secs(),
    })
}

pub fn export(conn: &Connection, root: &Path) -> Result<()> {
    let mut statement =
        conn.prepare("SELECT rel_path, stars, updated_at FROM image_rating ORDER BY rel_path")?;
    let ratings = statement
        .query_map([], |row| {
            Ok(SnapshotEntry {
                rel_path: row.get(0)?,
                stars: row.get(1)?,
                updated_at: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let snapshot = Snapshot {
        version: SNAPSHOT_VERSION,
        exported_at: now_secs(),
        ratings,
    };
    let bytes = serde_json::to_vec_pretty(&snapshot)?;
    write_atomic(&snapshot_path(root), &bytes)
}

pub fn restore(conn: &mut Connection, root: &Path) -> Result<u64> {
    let path = snapshot_path(root);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(error).with_context(|| format!("读取评级快照失败: {}", path.display()));
        }
    };
    let snapshot: Snapshot = serde_json::from_slice(&bytes)
        .with_context(|| format!("解析评级快照失败: {}", path.display()))?;
    let tx = conn.transaction()?;
    let mut restored = 0u64;
    for entry in snapshot.ratings {
        if !(1..=5).contains(&entry.stars) {
            continue;
        }
        let changed = tx.execute(
            "INSERT INTO image_rating(rel_path, stars, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(rel_path) DO UPDATE SET
               stars = excluded.stars,
               updated_at = excluded.updated_at
             WHERE excluded.updated_at > image_rating.updated_at",
            params![entry.rel_path, entry.stars, entry.updated_at],
        )?;
        restored += changed as u64;
    }
    tx.commit()?;
    Ok(restored)
}

pub fn snapshot_path(root: &Path) -> PathBuf {
    root.join(".pixtrace").join("ratings.json")
}

fn image_rel_path(conn: &Connection, id: i64) -> Result<String> {
    conn.query_row(
        "SELECT rel_path FROM image WHERE id = ?1 AND deleted_at IS NULL",
        [id],
        |row| row.get(0),
    )
    .context("图像不存在或已删除")
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, bytes)
        .with_context(|| format!("写入评级快照失败: {}", temp.display()))?;
    if std::fs::rename(&temp, path).is_err() {
        let _ = std::fs::remove_file(path);
        std::fs::rename(&temp, path)
            .with_context(|| format!("替换评级快照失败: {}", path.display()))?;
    }
    Ok(())
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
