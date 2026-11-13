use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::index::{db, writer};
use crate::metadata;
use crate::prompt::{self, Extracted, ParseStatus};

const BATCH_SIZE: u32 = 64;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    pub running: bool,
    pub total: u64,
    pub done: u64,
    pub failed: u64,
    pub phase: String,
}

impl Default for IndexProgress {
    fn default() -> Self {
        Self {
            running: false,
            total: 0,
            done: 0,
            failed: 0,
            phase: "空闲".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexChanged {
    done: u64,
    failed: u64,
}

pub struct Indexer {
    progress: Arc<Mutex<IndexProgress>>,
    cancel: Mutex<Option<Arc<AtomicBool>>>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl Default for Indexer {
    fn default() -> Self {
        Self::new()
    }
}

impl Indexer {
    pub fn new() -> Self {
        Self {
            progress: Arc::new(Mutex::new(IndexProgress::default())),
            cancel: Mutex::new(None),
            handle: Mutex::new(None),
        }
    }

    pub fn progress(&self) -> IndexProgress {
        self.progress
            .lock()
            .map(|progress| progress.clone())
            .unwrap_or_default()
    }

    pub fn start(&self, app: AppHandle, root: PathBuf, db_path: PathBuf) {
        self.stop();

        let cancel = Arc::new(AtomicBool::new(false));
        *self.cancel.lock().expect("cancel lock") = Some(cancel.clone());
        if let Ok(mut progress) = self.progress.lock() {
            *progress = IndexProgress {
                running: true,
                phase: "准备索引".to_string(),
                ..IndexProgress::default()
            };
        }
        emit_progress(&app, &self.progress);

        let progress = self.progress.clone();
        let handle = std::thread::Builder::new()
            .name("pixtrace-indexer".to_string())
            .spawn(move || run(app, root, db_path, cancel, progress))
            .ok();
        *self.handle.lock().expect("handle lock") = handle;
    }

    pub fn stop(&self) {
        if let Some(cancel) = self.cancel.lock().expect("cancel lock").take() {
            cancel.store(true, Ordering::SeqCst);
        }
        let handle = self.handle.lock().expect("handle lock").take();
        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }
}

struct PendingRow {
    id: i64,
    rel_path: String,
    format: String,
}

fn run(
    app: AppHandle,
    root: PathBuf,
    db_path: PathBuf,
    cancel: Arc<AtomicBool>,
    progress: Arc<Mutex<IndexProgress>>,
) {
    let result = run_inner(&app, &root, &db_path, &cancel, &progress);
    let (done, failed) = {
        let mut guard = progress.lock().expect("progress lock");
        guard.running = false;
        match &result {
            Ok(()) => {
                guard.phase = if guard.failed > 0 {
                    "索引完成（有失败项）".to_string()
                } else {
                    "索引完成".to_string()
                };
            }
            Err(error) => {
                tracing::error!(%error, "indexing failed");
                guard.phase = format!("索引失败: {error}");
            }
        }
        (guard.done, guard.failed)
    };
    emit_progress(&app, &progress);
    let _ = app.emit("index:changed", IndexChanged { done, failed });
}

fn run_inner(
    app: &AppHandle,
    root: &Path,
    db_path: &Path,
    cancel: &AtomicBool,
    progress: &Arc<Mutex<IndexProgress>>,
) -> Result<()> {
    let mut conn = db::open(db_path)?;
    let total: u64 = conn.query_row(
        "SELECT COUNT(*) FROM image WHERE parse_status = 'pending' AND deleted_at IS NULL",
        [],
        |row| row.get(0),
    )?;

    {
        let mut guard = progress.lock().expect("progress lock");
        guard.total = total;
        guard.done = 0;
        guard.failed = 0;
        guard.running = true;
        guard.phase = if total == 0 {
            "索引已是最新".to_string()
        } else {
            "解析元数据".to_string()
        };
    }
    emit_progress(app, progress);

    if total == 0 {
        return Ok(());
    }

    loop {
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        let rows = load_pending(&conn, BATCH_SIZE)?;
        if rows.is_empty() {
            break;
        }

        let mut parsed = Vec::with_capacity(rows.len());
        for row in &rows {
            if cancel.load(Ordering::SeqCst) {
                break;
            }
            let item = match parse_one(root, row) {
                Ok(item) => item,
                Err(error) => error_item(row, error),
            };
            parsed.push(item);
        }
        if parsed.is_empty() {
            break;
        }

        writer::write_batch(&mut conn, &parsed)?;
        let failed = parsed.iter().filter(|item| item.status == "error").count() as u64;
        {
            let mut guard = progress.lock().expect("progress lock");
            guard.done += parsed.len() as u64;
            guard.failed += failed;
        }
        emit_progress(app, progress);
    }

    Ok(())
}

fn load_pending(conn: &Connection, limit: u32) -> Result<Vec<PendingRow>> {
    let mut statement = conn.prepare(
        "SELECT id, rel_path, format FROM image
         WHERE parse_status = 'pending' AND deleted_at IS NULL
         ORDER BY id LIMIT ?1",
    )?;
    let rows = statement.query_map([limit], |row| {
        Ok(PendingRow {
            id: row.get(0)?,
            rel_path: row.get(1)?,
            format: row.get(2)?,
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

fn parse_one(root: &Path, row: &PendingRow) -> Result<writer::ParsedImage> {
    let relative = row.rel_path.replace('/', std::path::MAIN_SEPARATOR_STR);
    let path = root.join(relative);
    let canonical = path
        .canonicalize()
        .with_context(|| format!("文件不存在: {}", row.rel_path))?;
    if !canonical.starts_with(root) {
        bail!("路径越界: {}", row.rel_path);
    }

    let meta = metadata::parse(&canonical, &row.format)?;
    let mut extracted = prompt::extract(&meta);
    if meta.truncated && extracted.status == ParseStatus::Ok {
        extracted.status = ParseStatus::Partial;
        if extracted.error.is_none() {
            extracted.error = Some("PNG 数据被截断".to_string());
        }
    }

    let prompt_json = meta.text("prompt").map(writer::compress);
    let workflow_json = meta.text("workflow").map(writer::compress);
    let raw_text = if extracted.status == ParseStatus::Error {
        meta.text("prompt")
            .or_else(|| meta.text("workflow"))
            .or_else(|| meta.text("parameters"))
            .map(writer::compress)
    } else {
        None
    };

    Ok(writer::ParsedImage {
        id: row.id,
        width: extracted.width,
        height: extracted.height,
        status: extracted.status.as_str(),
        parse_error: extracted.error.clone(),
        extracted,
        prompt_json,
        workflow_json,
        raw_text,
        indexed_at: now_secs(),
    })
}

fn error_item(row: &PendingRow, error: anyhow::Error) -> writer::ParsedImage {
    tracing::warn!(id = row.id, path = %row.rel_path, %error, "parse failed");
    writer::ParsedImage {
        id: row.id,
        width: None,
        height: None,
        status: ParseStatus::Error.as_str(),
        parse_error: Some(error.to_string()),
        extracted: Extracted::default(),
        prompt_json: None,
        workflow_json: None,
        raw_text: None,
        indexed_at: now_secs(),
    }
}

fn emit_progress(app: &AppHandle, progress: &Arc<Mutex<IndexProgress>>) {
    if let Ok(snapshot) = progress.lock().map(|progress| progress.clone()) {
        let _ = app.emit("scan:progress", snapshot);
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
