use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::UNIX_EPOCH;

use anyhow::Result;
use rusqlite::{params, Connection};
use walkdir::{DirEntry, WalkDir};

const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub image_count: u64,
    pub changed: u64,
    pub deleted: u64,
}

#[derive(Debug)]
struct ExistingRow {
    id: i64,
    file_size: i64,
    mtime_ns: i64,
    deleted: bool,
}

pub fn scan(root: &Path, conn: &mut Connection) -> Result<ScanSummary> {
    let tx = conn.transaction()?;

    let mut existing: HashMap<String, ExistingRow> = HashMap::new();
    {
        let mut statement =
            tx.prepare("SELECT id, rel_path, file_size, mtime_ns, deleted_at FROM image")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                ExistingRow {
                    id: row.get(0)?,
                    file_size: row.get(2)?,
                    mtime_ns: row.get(3)?,
                    deleted: row.get::<_, Option<i64>>(4)?.is_some(),
                },
            ))
        })?;
        for row in rows {
            let (rel_path, row) = row?;
            existing.insert(rel_path, row);
        }
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut changed = 0u64;
    {
        let mut statement = tx.prepare(
            "INSERT INTO image (rel_path, file_size, mtime_ns, format, parse_status)
             VALUES (?1, ?2, ?3, ?4, 'pending')
             ON CONFLICT(rel_path) DO UPDATE SET
               file_size = excluded.file_size,
               mtime_ns = excluded.mtime_ns,
               format = excluded.format,
               deleted_at = NULL,
               parse_status = CASE
                 WHEN image.deleted_at IS NOT NULL THEN 'pending'
                 WHEN image.file_size = excluded.file_size AND image.mtime_ns = excluded.mtime_ns
                 THEN image.parse_status ELSE 'pending' END",
        )?;

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| !is_skipped(entry))
        {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
                continue;
            };
            let extension = extension.to_ascii_lowercase();
            if !SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
                continue;
            }
            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let rel_path = relative
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let file_size = metadata.len() as i64;
            let mtime_ns = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_nanos() as i64)
                .unwrap_or(0);
            let unchanged = existing.get(&rel_path).is_some_and(|row| {
                !row.deleted && row.file_size == file_size && row.mtime_ns == mtime_ns
            });
            if !unchanged {
                changed += 1;
            }
            seen.insert(rel_path.clone());
            statement.execute(params![rel_path, file_size, mtime_ns, extension])?;
        }
    }

    let mut deleted = 0u64;
    for (rel_path, row) in &existing {
        if row.deleted || seen.contains(rel_path) {
            continue;
        }
        tx.execute(
            "UPDATE image SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now_secs(), row.id],
        )?;
        tx.execute("DELETE FROM image_fts WHERE rowid = ?1", params![row.id])?;
        deleted += 1;
    }

    tx.commit()?;

    Ok(ScanSummary {
        image_count: count_images(conn)?,
        changed,
        deleted,
    })
}

fn is_skipped(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    name == ".pixtrace" || name.starts_with('.')
}

pub fn count_images(conn: &Connection) -> Result<u64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM image WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )?)
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
