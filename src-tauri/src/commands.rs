use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use fs2::FileExt;
use rusqlite::OptionalExtension;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::index::groups::{self, GroupPage, GroupParams};
use crate::index::query::{self, FilterOptions, SearchParams, SearchResult, Sort};
use crate::index::{db, writer};
use crate::indexer::{IndexProgress, Indexer};
use crate::paths::{Mode, Paths};
use crate::prompt;
use crate::ratings;
use crate::scanner;
use crate::settings::{self, Settings};
use crate::thumbs::{self, CacheStats};
use crate::watcher::WorkspaceWatcher;

pub struct Workspace {
    pub root: PathBuf,
    pub db_path: PathBuf,
    pub db: rusqlite::Connection,
    _lock: std::fs::File,
}

pub struct AppState {
    pub paths: Paths,
    pub settings: Mutex<Settings>,
    pub workspace: Mutex<Option<Workspace>>,
    pub indexer: Indexer,
    pub watcher: WorkspaceWatcher,
    pub rescan_lock: Mutex<()>,
}

impl AppState {
    pub fn new(paths: Paths, settings: Settings) -> Self {
        Self {
            paths,
            settings: Mutex::new(settings),
            workspace: Mutex::new(None),
            indexer: Indexer::new(),
            watcher: WorkspaceWatcher::new(),
            rescan_lock: Mutex::new(()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub mode: String,
    pub data_dir: String,
    pub log_dir: String,
    pub webview_data_dir: String,
    pub settings_file: String,
    pub protocol_base: String,
    pub last_workspace: Option<String>,
    pub warning: Option<String>,
    pub thumb_cache_limit_mb: u64,
    pub watch_workspace: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    pub root: String,
    pub image_count: u64,
    pub db_path: String,
    pub schema_version: i64,
    pub scanned_at: i64,
    pub warning: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDetail {
    pub id: i64,
    pub rel_path: String,
    pub file_size: i64,
    pub mtime_ns: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub format: String,
    pub parse_status: String,
    pub parse_error: Option<String>,
    pub positive_prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub model_name: Option<String>,
    pub loras: Option<String>,
    pub sampler: Option<String>,
    pub steps: Option<i64>,
    pub cfg: Option<f64>,
    pub seed: Option<i64>,
    pub has_prompt_json: bool,
    pub has_workflow_json: bool,
    pub has_raw_text: bool,
    pub indexed_at: Option<i64>,
    pub stars: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAsset {
    pub kind: String,
    pub text: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFailure {
    pub id: i64,
    pub rel_path: String,
    pub error: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResult {
    pub deleted: u64,
    pub failed: Vec<DeleteFailure>,
}

#[tauri::command]
pub fn get_app_info(state: State<'_, AppState>) -> AppInfo {
    let settings = state.settings.lock().expect("settings lock");
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        mode: match state.paths.mode {
            Mode::Portable => "portable",
            Mode::Installed => "installed",
        }
        .to_string(),
        data_dir: state.paths.data_dir.display().to_string(),
        log_dir: state.paths.log_dir.display().to_string(),
        webview_data_dir: state.paths.webview_data_dir.display().to_string(),
        settings_file: state.paths.settings_file.display().to_string(),
        protocol_base: state.paths.protocol_base.to_string(),
        last_workspace: settings.last_workspace.clone(),
        warning: state.paths.warning.clone(),
        thumb_cache_limit_mb: settings.thumb_cache_limit_mb,
        watch_workspace: settings.watch_workspace,
    }
}

#[tauri::command]
pub fn get_workspace_state(state: State<'_, AppState>) -> Result<Option<WorkspaceInfo>, String> {
    let workspace = state
        .workspace
        .lock()
        .map_err(|_| "工作区状态锁已损坏".to_string())?;
    match workspace.as_ref() {
        Some(workspace) => workspace_info(workspace)
            .map(Some)
            .map_err(|error| error.to_string()),
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn open_workspace(app: AppHandle, path: String) -> Result<WorkspaceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || open_workspace_impl(&app, PathBuf::from(path)))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn rescan_workspace(app: AppHandle) -> Result<WorkspaceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || rescan_current_impl(&app))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn rebuild_index(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || rebuild_index_impl(&app))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_cache_stats(state: State<'_, AppState>) -> Result<CacheStats, String> {
    let limit = state
        .settings
        .lock()
        .map(|settings| settings.thumb_cache_limit_mb)
        .unwrap_or(settings::DEFAULT_THUMB_CACHE_LIMIT_MB)
        .saturating_mul(1024 * 1024);
    let workspace = state
        .workspace
        .lock()
        .map_err(|_| "工作区状态锁已损坏".to_string())?;
    let Some(workspace) = workspace.as_ref() else {
        return Ok(CacheStats {
            limit_bytes: limit,
            ..CacheStats::default()
        });
    };
    thumbs::cache_stats(&workspace.root, &workspace.db_path, limit)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn clear_cache(state: State<'_, AppState>) -> Result<CacheStats, String> {
    let limit = state
        .settings
        .lock()
        .map(|settings| settings.thumb_cache_limit_mb)
        .unwrap_or(settings::DEFAULT_THUMB_CACHE_LIMIT_MB)
        .saturating_mul(1024 * 1024);
    let workspace = state
        .workspace
        .lock()
        .map_err(|_| "工作区状态锁已损坏".to_string())?;
    let Some(workspace) = workspace.as_ref() else {
        return Ok(CacheStats {
            limit_bytes: limit,
            ..CacheStats::default()
        });
    };
    thumbs::clear_cache(&workspace.root).map_err(|error| error.to_string())?;
    thumbs::cache_stats(&workspace.root, &workspace.db_path, limit)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_cache_limit(state: State<'_, AppState>, limit_mb: u64) -> Result<CacheStats, String> {
    let limit_mb = limit_mb.clamp(64, 102_400);
    {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "设置锁已损坏".to_string())?;
        settings.thumb_cache_limit_mb = limit_mb;
        settings::save(&state.paths.settings_file, &settings).map_err(|error| error.to_string())?;
    }
    let limit = limit_mb.saturating_mul(1024 * 1024);
    let workspace = state
        .workspace
        .lock()
        .map_err(|_| "工作区状态锁已损坏".to_string())?;
    let Some(workspace) = workspace.as_ref() else {
        return Ok(CacheStats {
            limit_bytes: limit,
            ..CacheStats::default()
        });
    };
    if let Err(error) = thumbs::enforce_limit(&workspace.root, limit) {
        tracing::warn!(%error, "thumbnail cache eviction failed");
    }
    thumbs::cache_stats(&workspace.root, &workspace.db_path, limit)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_watch_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    {
        let mut settings = state
            .settings
            .lock()
            .map_err(|_| "设置锁已损坏".to_string())?;
        settings.watch_workspace = enabled;
        settings::save(&state.paths.settings_file, &settings).map_err(|error| error.to_string())?;
    }
    if enabled {
        let root = {
            let workspace = state
                .workspace
                .lock()
                .map_err(|_| "工作区状态锁已损坏".to_string())?;
            workspace.as_ref().map(|workspace| workspace.root.clone())
        };
        if let Some(root) = root {
            state
                .watcher
                .start(app, root)
                .map_err(|error| error.to_string())?;
        }
    } else {
        state.watcher.stop();
    }
    Ok(enabled)
}

#[tauri::command]
pub async fn delete_images(
    app: AppHandle,
    ids: Vec<i64>,
    permanent: bool,
) -> Result<DeleteResult, String> {
    tauri::async_runtime::spawn_blocking(move || delete_images_impl(&app, ids, permanent))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_images(
    state: State<'_, AppState>,
    offset: Option<u32>,
    limit: Option<u32>,
) -> Result<SearchResult, String> {
    let workspace = state
        .workspace
        .lock()
        .map_err(|_| "工作区状态锁已损坏".to_string())?;
    let Some(workspace) = workspace.as_ref() else {
        return Ok(SearchResult {
            items: Vec::new(),
            total: 0,
        });
    };
    query::search(
        &workspace.db,
        &SearchParams {
            query: String::new(),
            sort: Sort::Path,
            offset: offset.unwrap_or(0),
            limit: limit.unwrap_or(200).clamp(1, 500),
            ..Default::default()
        },
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn search_images(
    state: State<'_, AppState>,
    params: SearchParams,
) -> Result<SearchResult, String> {
    let workspace = state
        .workspace
        .lock()
        .map_err(|_| "工作区状态锁已损坏".to_string())?;
    let Some(workspace) = workspace.as_ref() else {
        return Ok(SearchResult {
            items: Vec::new(),
            total: 0,
        });
    };
    let params = SearchParams {
        model: params.model.filter(|value| !value.is_empty()),
        sampler: params.sampler.filter(|value| !value.is_empty()),
        limit: params.limit.clamp(1, 500),
        ..params
    };
    query::search(&workspace.db, &params).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_filter_options(state: State<'_, AppState>) -> Result<FilterOptions, String> {
    let workspace = state
        .workspace
        .lock()
        .map_err(|_| "工作区状态锁已损坏".to_string())?;
    let Some(workspace) = workspace.as_ref() else {
        return Ok(FilterOptions::default());
    };
    query::filter_options(&workspace.db).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_prompt_groups(
    state: State<'_, AppState>,
    params: GroupParams,
) -> Result<GroupPage, String> {
    let workspace = state
        .workspace
        .lock()
        .map_err(|_| "工作区状态锁已损坏".to_string())?;
    let Some(workspace) = workspace.as_ref() else {
        return Ok(GroupPage {
            items: Vec::new(),
            total: 0,
        });
    };
    groups::list(&workspace.db, &params).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_rating(
    state: State<'_, AppState>,
    id: i64,
    stars: u8,
) -> Result<ratings::RatingResult, String> {
    let workspace = state
        .workspace
        .lock()
        .map_err(|_| "工作区状态锁已损坏".to_string())?;
    let Some(workspace) = workspace.as_ref() else {
        return Err("尚未打开工作目录".to_string());
    };
    let result = ratings::set(&workspace.db, id, stars).map_err(|error| error.to_string())?;
    if let Err(error) = ratings::export(&workspace.db, &workspace.root) {
        tracing::warn!(%error, "rating snapshot export failed");
    }
    Ok(result)
}

#[tauri::command]
pub fn clear_rating(state: State<'_, AppState>, id: i64) -> Result<ratings::RatingResult, String> {
    let workspace = state
        .workspace
        .lock()
        .map_err(|_| "工作区状态锁已损坏".to_string())?;
    let Some(workspace) = workspace.as_ref() else {
        return Err("尚未打开工作目录".to_string());
    };
    let result = ratings::clear(&workspace.db, id).map_err(|error| error.to_string())?;
    if let Err(error) = ratings::export(&workspace.db, &workspace.root) {
        tracing::warn!(%error, "rating snapshot export failed");
    }
    Ok(result)
}

#[tauri::command]
pub fn get_image_detail(state: State<'_, AppState>, id: i64) -> Result<ImageDetail, String> {
    let workspace = state
        .workspace
        .lock()
        .map_err(|_| "工作区状态锁已损坏".to_string())?;
    let Some(workspace) = workspace.as_ref() else {
        return Err("尚未打开工作目录".to_string());
    };

    let detail = workspace
        .db
        .query_row(
            "SELECT i.id, i.rel_path, i.file_size, i.mtime_ns, i.width, i.height,
                    i.format, i.parse_status, i.parse_error, i.positive_prompt,
                    i.negative_prompt, i.model_name, i.loras, i.sampler, i.steps,
                    i.cfg, i.seed, i.indexed_at,
                    a.prompt_json IS NOT NULL, a.workflow_json IS NOT NULL,
                    a.raw_text IS NOT NULL,
                    (SELECT stars FROM image_rating WHERE rel_path = i.rel_path)
             FROM image i
             LEFT JOIN image_asset a ON a.image_id = i.id
             WHERE i.id = ?1 AND i.deleted_at IS NULL",
            [id],
            |row| {
                Ok(ImageDetail {
                    id: row.get(0)?,
                    rel_path: row.get(1)?,
                    file_size: row.get(2)?,
                    mtime_ns: row.get(3)?,
                    width: row.get(4)?,
                    height: row.get(5)?,
                    format: row.get(6)?,
                    parse_status: row.get(7)?,
                    parse_error: row.get(8)?,
                    positive_prompt: row.get(9)?,
                    negative_prompt: row.get(10)?,
                    model_name: row.get(11)?,
                    loras: row.get(12)?,
                    sampler: row.get(13)?,
                    steps: row.get(14)?,
                    cfg: row.get(15)?,
                    seed: row.get(16)?,
                    indexed_at: row.get(17)?,
                    has_prompt_json: row.get(18)?,
                    has_workflow_json: row.get(19)?,
                    has_raw_text: row.get(20)?,
                    stars: row.get(21)?,
                })
            },
        )
        .map_err(|error| error.to_string())?;
    Ok(detail)
}

#[tauri::command]
pub fn get_image_asset(
    state: State<'_, AppState>,
    id: i64,
    kind: String,
) -> Result<ImageAsset, String> {
    let workspace = state
        .workspace
        .lock()
        .map_err(|_| "工作区状态锁已损坏".to_string())?;
    let Some(workspace) = workspace.as_ref() else {
        return Err("尚未打开工作目录".to_string());
    };

    let column = match kind.as_str() {
        "prompt" => "prompt_json",
        "workflow" => "workflow_json",
        "raw" => "raw_text",
        _ => return Err(format!("未知的资源类型: {kind}")),
    };
    let query = format!("SELECT {column} FROM image_asset WHERE image_id = ?1");
    let blob: Option<Vec<u8>> = workspace
        .db
        .query_row(&query, [id], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let text = blob
        .map(|blob| writer::decompress(&blob))
        .transpose()
        .map_err(|error| error.to_string())?
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
    Ok(ImageAsset { kind, text })
}

#[tauri::command]
pub fn get_index_progress(state: State<'_, AppState>) -> IndexProgress {
    state.indexer.progress()
}

#[tauri::command]
pub fn cancel_indexing(state: State<'_, AppState>) {
    state.indexer.stop();
}

pub fn acquire_workspace_lock(pixtrace_dir: &Path) -> anyhow::Result<std::fs::File> {
    std::fs::create_dir_all(pixtrace_dir).with_context(|| {
        format!(
            "工作目录不可写，无法创建 .pixtrace: {}",
            pixtrace_dir.display()
        )
    })?;
    let lock_path = pixtrace_dir.join("workspace.lock");
    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .with_context(|| format!("创建工作目录锁失败: {}", lock_path.display()))?;
    lock_file.try_lock_exclusive().map_err(|_| {
        anyhow::anyhow!("工作目录已被另一个 PixTrace 实例打开。请先关闭该实例，或选择其他目录。")
    })?;
    let owner = format!("pid={}\nstarted_at={}\n", std::process::id(), now_secs());
    if let Err(error) = std::fs::write(pixtrace_dir.join("workspace.owner"), owner) {
        tracing::warn!(%error, "workspace owner info write failed");
    }
    Ok(lock_file)
}

fn resolve_for_delete(root: &Path, rel_path: &str) -> Option<PathBuf> {
    let relative = Path::new(rel_path);
    if relative.is_absolute() {
        return None;
    }
    let mut path = root.to_path_buf();
    for component in relative.components() {
        match component {
            Component::Normal(part) => path.push(part),
            _ => return None,
        }
    }
    let canonical_root = root.canonicalize().ok()?;
    let parent = path.parent()?.canonicalize().ok()?;
    parent.starts_with(&canonical_root).then_some(path)
}

fn delete_images_impl(
    app: &AppHandle,
    mut ids: Vec<i64>,
    permanent: bool,
) -> anyhow::Result<DeleteResult> {
    ids.sort_unstable();
    ids.dedup();

    let state = app.state::<AppState>();
    let _guard = state
        .rescan_lock
        .lock()
        .map_err(|_| anyhow::anyhow!("索引任务锁已损坏"))?;
    state.indexer.stop();

    let (root, targets) = {
        let workspace = state
            .workspace
            .lock()
            .map_err(|_| anyhow::anyhow!("工作区状态锁已损坏"))?;
        let Some(workspace) = workspace.as_ref() else {
            anyhow::bail!("尚未打开工作目录");
        };
        let mut targets = Vec::with_capacity(ids.len());
        for id in ids {
            let rel_path = workspace
                .db
                .query_row(
                    "SELECT rel_path FROM image WHERE id = ?1 AND deleted_at IS NULL",
                    [id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            if let Some(rel_path) = rel_path {
                targets.push((id, rel_path));
            }
        }
        (workspace.root.clone(), targets)
    };

    let mut deleted_ids = Vec::new();
    let mut failed = Vec::new();
    for (id, rel_path) in targets {
        let Some(path) = resolve_for_delete(&root, &rel_path) else {
            failed.push(DeleteFailure {
                id,
                rel_path,
                error: "文件不存在或路径无效".to_string(),
            });
            continue;
        };
        let result = if !path.exists() {
            Ok(())
        } else if permanent {
            std::fs::remove_file(&path).map_err(|error| error.to_string())
        } else {
            trash::delete(&path).map_err(|error| error.to_string())
        };
        match result {
            Ok(()) => deleted_ids.push(id),
            Err(error) => failed.push(DeleteFailure {
                id,
                rel_path,
                error,
            }),
        }
    }

    let deleted = deleted_ids.len() as u64;
    let (info, root, db_path, pending) = {
        let mut workspace = state
            .workspace
            .lock()
            .map_err(|_| anyhow::anyhow!("工作区状态锁已损坏"))?;
        let Some(workspace) = workspace.as_mut() else {
            anyhow::bail!("尚未打开工作目录");
        };
        if !deleted_ids.is_empty() {
            let now = now_secs();
            let tx = workspace.db.transaction()?;
            for id in &deleted_ids {
                tx.execute(
                    "UPDATE image SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
                    rusqlite::params![now, id],
                )?;
                tx.execute("DELETE FROM image_fts WHERE rowid = ?1", [id])?;
            }
            tx.execute(
                "INSERT INTO meta(key, value) VALUES('last_scan_at', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![now.to_string()],
            )?;
            tx.commit()?;
        }
        let pending: i64 = workspace.db.query_row(
            "SELECT COUNT(*) FROM image
             WHERE parse_status = 'pending' AND deleted_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        (
            workspace_info(workspace)?,
            workspace.root.clone(),
            workspace.db_path.clone(),
            pending,
        )
    };

    if pending > 0 {
        state.indexer.start(app.clone(), root, db_path);
    }
    if deleted > 0 {
        if let Err(error) = app.emit("workspace:changed", &info) {
            tracing::warn!(%error, "workspace change event failed");
        }
        if let Err(error) = app.emit("index:changed", ()) {
            tracing::warn!(%error, "index change event failed");
        }
    }
    Ok(DeleteResult { deleted, failed })
}

fn open_workspace_impl(app: &AppHandle, path: PathBuf) -> anyhow::Result<WorkspaceInfo> {
    let root = path
        .canonicalize()
        .with_context(|| format!("工作目录不存在: {}", path.display()))?;
    if !root.is_dir() {
        anyhow::bail!("不是目录: {}", root.display());
    }

    {
        let state = app.state::<AppState>();
        let same_root = {
            let workspace = state
                .workspace
                .lock()
                .map_err(|_| anyhow::anyhow!("工作区状态锁已损坏"))?;
            workspace
                .as_ref()
                .is_some_and(|workspace| workspace.root == root)
        };
        if same_root {
            return rescan_current_impl(app);
        }
    }

    let state = app.state::<AppState>();
    state.watcher.stop();
    state.indexer.stop();
    let result = open_workspace_after_root(app, root);
    if result.is_err() {
        restart_workspace_services(app);
    }
    result
}

fn open_workspace_after_root(app: &AppHandle, root: PathBuf) -> anyhow::Result<WorkspaceInfo> {
    let state = app.state::<AppState>();
    let _rescan_guard = state
        .rescan_lock
        .lock()
        .map_err(|_| anyhow::anyhow!("索引任务锁已损坏"))?;

    let pixtrace_dir = root.join(".pixtrace");
    let lock_file = acquire_workspace_lock(&pixtrace_dir)?;

    let db_path = pixtrace_dir.join("index.db");
    let mut conn = db::open(&db_path)?;
    let summary = scanner::scan(&root, &mut conn)?;
    match ratings::restore(&mut conn, &root) {
        Ok(restored) if restored > 0 => {
            tracing::info!(restored, "ratings restored from snapshot");
        }
        Ok(_) => {}
        Err(error) => tracing::warn!(%error, "rating snapshot restore failed"),
    }
    if let Err(error) = ratings::export(&conn, &root) {
        tracing::warn!(%error, "rating snapshot export failed");
    }

    let stored_parser: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'parser_version'",
            [],
            |row| row.get(0),
        )
        .ok();
    if stored_parser.as_deref() != Some(prompt::PARSER_VERSION) {
        conn.execute(
            "UPDATE image SET parse_status = 'pending' WHERE deleted_at IS NULL",
            [],
        )?;
        conn.execute(
            "INSERT INTO meta(key, value) VALUES('parser_version', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![prompt::PARSER_VERSION],
        )?;
    }

    let now = now_secs();
    conn.execute(
        "INSERT INTO meta(key, value) VALUES('workdir_root', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![display_path(&root)],
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES('last_scan_at', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![now.to_string()],
    )?;

    let mut info = WorkspaceInfo {
        root: display_path(&root),
        image_count: summary.image_count,
        db_path: db_path.display().to_string(),
        schema_version: db::schema_version(&conn)?,
        scanned_at: now,
        warning: workspace_warning(&root),
    };

    {
        let mut settings = state.settings.lock().expect("settings lock");
        settings.last_workspace = Some(info.root.clone());
        let _ = settings::save(&state.paths.settings_file, &settings);
    }
    let watch_enabled = state
        .settings
        .lock()
        .map(|settings| settings.watch_workspace)
        .unwrap_or(true);

    *state.workspace.lock().expect("workspace lock") = Some(Workspace {
        root: root.clone(),
        db_path: db_path.clone(),
        db: conn,
        _lock: lock_file,
    });
    state.indexer.start(app.clone(), root.clone(), db_path);
    if watch_enabled {
        if let Err(error) = state.watcher.start(app.clone(), root) {
            tracing::warn!(%error, "workspace watcher failed to start");
            info.warning = merge_warning(
                info.warning,
                Some("文件监听启动失败，可使用“重新扫描”手动刷新。".to_string()),
            );
        }
    }

    tracing::info!(
        images = info.image_count,
        changed = summary.changed,
        deleted = summary.deleted,
        "workspace opened"
    );
    Ok(info)
}

fn restart_workspace_services(app: &AppHandle) {
    let state = app.state::<AppState>();
    let target = {
        match state.workspace.lock() {
            Ok(workspace) => workspace
                .as_ref()
                .map(|workspace| (workspace.root.clone(), workspace.db_path.clone())),
            Err(_) => None,
        }
    };
    let Some((root, db_path)) = target else {
        return;
    };
    state.indexer.start(app.clone(), root.clone(), db_path);
    let watch_enabled = state
        .settings
        .lock()
        .map(|settings| settings.watch_workspace)
        .unwrap_or(true);
    if watch_enabled {
        if let Err(error) = state.watcher.start(app.clone(), root) {
            tracing::warn!(%error, "failed to restart workspace services");
        }
    }
}

pub fn rescan_current(app: &AppHandle) -> anyhow::Result<WorkspaceInfo> {
    rescan_current_impl(app)
}

fn rescan_current_impl(app: &AppHandle) -> anyhow::Result<WorkspaceInfo> {
    let state = app.state::<AppState>();
    let _guard = state
        .rescan_lock
        .lock()
        .map_err(|_| anyhow::anyhow!("索引任务锁已损坏"))?;
    state.indexer.stop();

    let (info, root, db_path, needs_index) = {
        let mut workspace = state
            .workspace
            .lock()
            .map_err(|_| anyhow::anyhow!("工作区状态锁已损坏"))?;
        let Some(workspace) = workspace.as_mut() else {
            anyhow::bail!("尚未打开工作目录");
        };
        let summary = scanner::scan(&workspace.root, &mut workspace.db)?;
        let now = now_secs();
        workspace.db.execute(
            "INSERT INTO meta(key, value) VALUES('last_scan_at', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![now.to_string()],
        )?;
        let pending: i64 = workspace.db.query_row(
            "SELECT COUNT(*) FROM image
             WHERE parse_status = 'pending' AND deleted_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        let info = workspace_info(workspace)?;
        let needs_index = summary.changed > 0 || summary.deleted > 0 || pending > 0;
        (
            info,
            workspace.root.clone(),
            workspace.db_path.clone(),
            needs_index,
        )
    };

    if needs_index {
        state.indexer.start(app.clone(), root, db_path);
    }
    if let Err(error) = app.emit("workspace:changed", &info) {
        tracing::warn!(%error, "workspace change event failed");
    }
    Ok(info)
}

fn rebuild_index_impl(app: &AppHandle) -> anyhow::Result<()> {
    let state = app.state::<AppState>();
    let _guard = state
        .rescan_lock
        .lock()
        .map_err(|_| anyhow::anyhow!("索引任务锁已损坏"))?;
    state.indexer.stop();
    let (root, db_path) = {
        let workspace = state
            .workspace
            .lock()
            .map_err(|_| anyhow::anyhow!("工作区状态锁已损坏"))?;
        let Some(workspace) = workspace.as_ref() else {
            anyhow::bail!("尚未打开工作目录");
        };
        workspace.db.execute(
            "UPDATE image SET parse_status = 'pending' WHERE deleted_at IS NULL",
            [],
        )?;
        workspace.db.execute("DELETE FROM image_fts", [])?;
        (workspace.root.clone(), workspace.db_path.clone())
    };
    state.indexer.start(app.clone(), root, db_path);
    Ok(())
}

fn workspace_info(workspace: &Workspace) -> anyhow::Result<WorkspaceInfo> {
    let scanned_at = workspace
        .db
        .query_row(
            "SELECT value FROM meta WHERE key = 'last_scan_at'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    Ok(WorkspaceInfo {
        root: display_path(&workspace.root),
        image_count: scanner::count_images(&workspace.db)?,
        db_path: workspace.db_path.display().to_string(),
        schema_version: db::schema_version(&workspace.db)?,
        scanned_at,
        warning: workspace_warning(&workspace.root),
    })
}

fn workspace_warning(root: &std::path::Path) -> Option<String> {
    let text = display_path(root);
    if text.starts_with(r"\\") {
        Some("当前工作目录位于网络路径，文件监听可能不可靠；可使用“重新扫描”手动刷新。".to_string())
    } else {
        None
    }
}

fn merge_warning(first: Option<String>, second: Option<String>) -> Option<String> {
    match (first, second) {
        (Some(first), Some(second)) => Some(format!("{first} {second}")),
        (Some(first), None) => Some(first),
        (None, Some(second)) => Some(second),
        (None, None) => None,
    }
}

fn display_path(path: &std::path::Path) -> String {
    let text = path.display().to_string();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = text.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        text
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{merge_warning, resolve_for_delete, workspace_warning};
    use std::path::Path;

    #[test]
    fn delete_paths_stay_inside_workspace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("nested")).unwrap();
        std::fs::write(dir.path().join("nested").join("a.png"), b"x").unwrap();

        let resolved = resolve_for_delete(dir.path(), "nested/a.png").unwrap();
        assert_eq!(resolved, dir.path().join("nested").join("a.png"));
        assert!(resolve_for_delete(dir.path(), "../outside.png").is_none());
        assert!(resolve_for_delete(dir.path(), "/absolute.png").is_none());
    }

    #[test]
    fn delete_paths_allow_missing_files_in_existing_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let resolved = resolve_for_delete(dir.path(), "missing.png").unwrap();
        assert!(!resolved.exists());
    }

    #[test]
    fn network_paths_receive_warning() {
        let warning = workspace_warning(Path::new(r"\\server\share\outputs"));
        assert!(warning.unwrap().contains("网络路径"));
    }

    #[test]
    fn local_paths_have_no_warning() {
        assert!(workspace_warning(Path::new(r"C:\outputs")).is_none());
    }

    #[test]
    fn warnings_are_merged_without_duplicates() {
        assert_eq!(merge_warning(None, None), None);
        assert_eq!(merge_warning(Some("a".into()), None).as_deref(), Some("a"));
        assert_eq!(
            merge_warning(Some("a".into()), Some("b".into())).as_deref(),
            Some("a b")
        );
    }
}
