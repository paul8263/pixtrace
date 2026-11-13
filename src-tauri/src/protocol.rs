use std::path::{Path, PathBuf};

use tauri::http::{Request, Response, StatusCode};
use tauri::{AppHandle, Manager, Runtime};

use crate::commands::AppState;
use crate::thumbs;

const TEST_PNG: &[u8] = include_bytes!("../assets/protocol-test.png");

pub fn handle<R: Runtime>(app: &AppHandle<R>, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let path = request.uri().path().trim_start_matches('/');
    let segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    match segments.as_slice() {
        ["test"] => response(StatusCode::OK, "image/png", TEST_PNG.to_vec(), "no-cache"),
        ["image", id] => match id.parse::<i64>() {
            Ok(id) => serve_image(app, id),
            Err(_) => not_found("invalid image id"),
        },
        ["thumb", id] => match id.parse::<i64>() {
            Ok(id) => {
                let size = query_param(request.uri().query(), "size")
                    .and_then(|value| value.parse::<u32>().ok())
                    .unwrap_or(thumbs::GRID_THUMB_SIZE);
                serve_thumb(app, id, size)
            }
            Err(_) => not_found("invalid image id"),
        },
        _ => not_found("not found"),
    }
}

fn serve_image<R: Runtime>(app: &AppHandle<R>, id: i64) -> Response<Vec<u8>> {
    let Some((root, rel_path)) = image_location(app, id) else {
        return not_found("image not found");
    };
    let Some(path) = resolve_under_root(&root, &rel_path) else {
        return forbidden("path outside workspace");
    };
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(_) => return not_found("image file missing"),
    };
    response(
        StatusCode::OK,
        mime_for(&path),
        bytes,
        "public, max-age=31536000, immutable",
    )
}

fn serve_thumb<R: Runtime>(app: &AppHandle<R>, id: i64, size: u32) -> Response<Vec<u8>> {
    let state = app.state::<AppState>();
    let (root, rel_path, file_size, mtime_ns) = {
        let workspace = match state.workspace.lock() {
            Ok(workspace) => workspace,
            Err(_) => return internal_error("workspace lock poisoned"),
        };
        let Some(workspace) = workspace.as_ref() else {
            return not_found("no workspace");
        };
        let row = workspace.db.query_row(
            "SELECT rel_path, file_size, mtime_ns FROM image
             WHERE id = ?1 AND deleted_at IS NULL",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        );
        match row {
            Ok((rel_path, file_size, mtime_ns)) => {
                (workspace.root.clone(), rel_path, file_size, mtime_ns)
            }
            Err(_) => return not_found("image not found"),
        }
    };

    let Some(source) = resolve_under_root(&root, &rel_path) else {
        return forbidden("path outside workspace");
    };
    let limit_bytes = state
        .settings
        .lock()
        .map(|settings| settings.thumb_cache_limit_mb)
        .unwrap_or(crate::settings::DEFAULT_THUMB_CACHE_LIMIT_MB)
        .saturating_mul(1024 * 1024);
    let cache_path = match thumbs::ensure_thumb(
        &root,
        &source,
        &rel_path,
        file_size,
        mtime_ns,
        size,
        limit_bytes,
    ) {
        Ok(path) => path,
        Err(error) => {
            tracing::warn!(%error, id, "thumbnail generation failed");
            return internal_error("thumbnail generation failed");
        }
    };
    match std::fs::read(&cache_path) {
        Ok(bytes) => response(
            StatusCode::OK,
            "image/webp",
            bytes,
            "public, max-age=31536000, immutable",
        ),
        Err(_) => internal_error("thumbnail missing"),
    }
}

fn image_location<R: Runtime>(app: &AppHandle<R>, id: i64) -> Option<(PathBuf, String)> {
    let state = app.state::<AppState>();
    let workspace = state.workspace.lock().ok()?;
    let workspace = workspace.as_ref()?;
    let rel_path = workspace
        .db
        .query_row(
            "SELECT rel_path FROM image WHERE id = ?1 AND deleted_at IS NULL",
            [id],
            |row| row.get::<_, String>(0),
        )
        .ok()?;
    Some((workspace.root.clone(), rel_path))
}

pub fn resolve_under_root(root: &Path, relative: &str) -> Option<PathBuf> {
    let canonical_root = root.canonicalize().ok()?;
    let canonical = canonical_root.join(relative).canonicalize().ok()?;
    canonical.starts_with(&canonical_root).then_some(canonical)
}

fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

fn query_param<'a>(query: Option<&'a str>, key: &str) -> Option<&'a str> {
    query?
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| *name == key)
        .map(|(_, value)| value)
}

fn response(
    status: StatusCode,
    mime: &str,
    body: Vec<u8>,
    cache_control: &str,
) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header("Content-Type", mime)
        .header("Cache-Control", cache_control)
        .body(body)
        .expect("valid response")
}

fn not_found(message: &str) -> Response<Vec<u8>> {
    response(
        StatusCode::NOT_FOUND,
        "text/plain; charset=utf-8",
        message.as_bytes().to_vec(),
        "no-cache",
    )
}

fn forbidden(message: &str) -> Response<Vec<u8>> {
    response(
        StatusCode::FORBIDDEN,
        "text/plain; charset=utf-8",
        message.as_bytes().to_vec(),
        "no-cache",
    )
}

fn internal_error(message: &str) -> Response<Vec<u8>> {
    response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "text/plain; charset=utf-8",
        message.as_bytes().to_vec(),
        "no-cache",
    )
}

#[cfg(test)]
mod tests {
    use super::resolve_under_root;

    #[test]
    fn accepts_paths_inside_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("inside.png"), b"x").unwrap();
        assert!(resolve_under_root(dir.path(), "inside.png").is_some());
    }

    #[test]
    fn rejects_paths_outside_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(dir.path().join("outside.png"), b"x").unwrap();
        assert!(resolve_under_root(&root, "../outside.png").is_none());
    }
}
