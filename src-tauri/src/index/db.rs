use std::path::Path;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use rusqlite::Connection;

pub const SCHEMA_VERSION: i64 = 1;

const SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS image (
  id               INTEGER PRIMARY KEY,
  rel_path         TEXT    NOT NULL UNIQUE,
  file_size        INTEGER NOT NULL,
  mtime_ns         INTEGER NOT NULL,
  width            INTEGER,
  height           INTEGER,
  format           TEXT    NOT NULL DEFAULT 'png',
  parse_status     TEXT    NOT NULL DEFAULT 'pending',
  parse_error      TEXT,
  positive_prompt  TEXT,
  negative_prompt  TEXT,
  prompt_hash      TEXT,
  search_positive  TEXT,
  search_negative  TEXT,
  model_name       TEXT,
  loras            TEXT,
  sampler          TEXT,
  steps            INTEGER,
  cfg              REAL,
  seed             INTEGER,
  indexed_at       INTEGER,
  deleted_at       INTEGER
);

CREATE INDEX IF NOT EXISTS idx_image_prompt_hash
  ON image(prompt_hash) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_image_mtime
  ON image(mtime_ns) WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS image_asset (
  image_id      INTEGER PRIMARY KEY REFERENCES image(id) ON DELETE CASCADE,
  prompt_json   BLOB,
  workflow_json BLOB,
  raw_text      BLOB
);

CREATE TABLE IF NOT EXISTS image_rating (
  rel_path   TEXT PRIMARY KEY,
  stars      INTEGER NOT NULL CHECK (stars BETWEEN 1 AND 5),
  updated_at INTEGER NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS image_fts USING fts5(
  positive,
  negative,
  model,
  loras,
  tokenize = 'unicode61 remove_diacritics 2'
);
"#;

pub fn open(path: &Path) -> Result<Connection> {
    let conn =
        Connection::open(path).with_context(|| format!("打开数据库失败: {}", path.display()))?;
    configure(&conn)?;
    migrate(&conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> Result<()> {
    if conn
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get::<_, String>(0))
        .is_err()
    {
        let _ = conn.query_row("PRAGMA journal_mode=DELETE", [], |row| {
            row.get::<_, String>(0)
        });
    }
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(Duration::from_secs(5))?;
    Ok(())
}

pub fn migrate(conn: &Connection) -> Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > SCHEMA_VERSION {
        bail!("索引版本 {} 高于当前程序支持的 {}", version, SCHEMA_VERSION);
    }
    if version < SCHEMA_VERSION {
        conn.execute_batch(SCHEMA_V1)?;
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }
    Ok(())
}

pub fn schema_version(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}
