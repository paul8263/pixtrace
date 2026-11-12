use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::index::db;
use crate::paths::{Mode, Paths};
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
}

impl AppState {
    pub fn new(paths: Paths, settings: Settings) -> Self {
        Self {
            paths,
            settings: Mutex::new(settings),
            workspace: Mutex::new(None),
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
pub struct ImageSummary {
    pub id: i64,
    pub rel_path: String,
    pub file_size: i64,
    pub mtime_ns: i64,
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
    limit: Option<u32>,
) -> Result<Vec<ImageSummary>, String> {
    let workspace = state
        .workspace
        .lock()
        .map_err(|_| "工作区状态锁已损坏".to_string())?;
    let Some(workspace) = workspace.as_ref() else {
        return Ok(Vec::new());
    };
    let limit = limit.unwrap_or(24).clamp(1, 200) as i64;
    let mut statement = workspace
        .db
        .prepare(
            "SELECT id, rel_path, file_size, mtime_ns
             FROM image WHERE deleted_at IS NULL
             ORDER BY rel_path LIMIT ?1",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(rusqlite::params![limit], |row| {
            Ok(ImageSummary {
                id: row.get(0)?,
                rel_path: row.get(1)?,
                file_size: row.get(2)?,
                mtime_ns: row.get(3)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

fn open_workspace_impl(app: &AppHandle, path: PathBuf) -> anyhow::Result<WorkspaceInfo> {
    let root = path
        .canonicalize()
        .with_context(|| format!("工作目录不存在: {}", path.display()))?;
    if !root.is_dir() {
        anyhow::bail!("不是目录: {}", root.display());
    }

    let pixtrace_dir = root.join(".pixtrace");
    std::fs::create_dir_all(&pixtrace_dir)
        .with_context(|| format!("创建工作目录索引目录失败: {}", pixtrace_dir.display()))?;
    let db_path = pixtrace_dir.join("index.db");
    let mut conn = db::open(&db_path)?;
    let summary = scanner::scan(&root, &mut conn)?;
    let now = now_secs();

    conn.execute(
        "INSERT INTO meta(key, value) VALUES('workdir_root', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![root.display().to_string()],
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES('last_scan_at', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![now.to_string()],
    )?;

    let info = WorkspaceInfo {
        root: root.display().to_string(),
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
        root,
        db_path,
        db: conn,
    });

    tracing::info!(
        images = info.image_count,
        changed = summary.changed,
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
        root: workspace.root.display().to_string(),
        image_count: scanner::count_images(&workspace.db)?,
        db_path: workspace.db_path.display().to_string(),
        schema_version: db::schema_version(&workspace.db)?,
        scanned_at,
    })
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
