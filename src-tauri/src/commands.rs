use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::index::query::{self, FilterOptions, SearchParams, SearchResult, Sort};
use crate::index::{db, writer};
use crate::indexer::{IndexProgress, Indexer};
use crate::paths::{Mode, Paths};
use crate::prompt;
use crate::scanner;
use crate::settings::{self, Settings};

pub struct Workspace {
    pub root: PathBuf,
    pub db_path: PathBuf,
    pub db: rusqlite::Connection,
}

pub struct AppState {
    pub paths: Paths,
    pub settings: Mutex<Settings>,
    pub workspace: Mutex<Option<Workspace>>,
    pub indexer: Indexer,
}

impl AppState {
    pub fn new(paths: Paths, settings: Settings) -> Self {
        Self {
            paths,
            settings: Mutex::new(settings),
            workspace: Mutex::new(None),
            indexer: Indexer::new(),
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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    pub root: String,
    pub image_count: u64,
    pub db_path: String,
    pub schema_version: i64,
    pub scanned_at: i64,
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAsset {
    pub kind: String,
    pub text: Option<String>,
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
    tauri::async_runtime::spawn_blocking(move || {
        let root = {
            let state = app.state::<AppState>();
            let workspace = state.workspace.lock().expect("workspace lock");
            workspace
                .as_ref()
                .map(|workspace| workspace.root.clone())
                .ok_or_else(|| anyhow::anyhow!("尚未打开工作目录"))?
        };
        open_workspace_impl(&app, root)
    })
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
            model: None,
            sampler: None,
            has_workflow: false,
            sort: Sort::Path,
            offset: offset.unwrap_or(0),
            limit: limit.unwrap_or(200).clamp(1, 500),
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
                    a.raw_text IS NOT NULL
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

fn open_workspace_impl(app: &AppHandle, path: PathBuf) -> anyhow::Result<WorkspaceInfo> {
    let root = path
        .canonicalize()
        .with_context(|| format!("工作目录不存在: {}", path.display()))?;
    if !root.is_dir() {
        anyhow::bail!("不是目录: {}", root.display());
    }

    {
        let state = app.state::<AppState>();
        state.indexer.stop();
    }

    let pixtrace_dir = root.join(".pixtrace");
    std::fs::create_dir_all(&pixtrace_dir)
        .with_context(|| format!("创建工作目录索引目录失败: {}", pixtrace_dir.display()))?;
    let db_path = pixtrace_dir.join("index.db");
    let mut conn = db::open(&db_path)?;
    let summary = scanner::scan(&root, &mut conn)?;

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

    let info = WorkspaceInfo {
        root: display_path(&root),
        image_count: summary.image_count,
        db_path: db_path.display().to_string(),
        schema_version: db::schema_version(&conn)?,
        scanned_at: now,
    };

    let state = app.state::<AppState>();
    {
        let mut settings = state.settings.lock().expect("settings lock");
        settings.last_workspace = Some(info.root.clone());
        let _ = settings::save(&state.paths.settings_file, &settings);
    }
    *state.workspace.lock().expect("workspace lock") = Some(Workspace {
        root: root.clone(),
        db_path: db_path.clone(),
        db: conn,
    });
    state.indexer.start(app.clone(), root, db_path);

    tracing::info!(
        images = info.image_count,
        changed = summary.changed,
        deleted = summary.deleted,
        "workspace opened"
    );
    Ok(info)
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
    })
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
