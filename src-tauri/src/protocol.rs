use std::path::{Path, PathBuf};

use tauri::http::{Request, Response, StatusCode};
use tauri::{AppHandle, Manager, Runtime};

use crate::commands::AppState;

const TEST_PNG: &[u8] = include_bytes!("../assets/protocol-test.png");

pub fn handle<R: Runtime>(app: &AppHandle<R>, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let path = request.uri().path().trim_start_matches('/');
    let segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    match segments.as_slice() {
        ["test"] => response(StatusCode::OK, "image/png", TEST_PNG.to_vec()),
        ["image", id] => match id.parse::<i64>() {
            Ok(id) => serve_image(app, id),
            Err(_) => not_found("invalid image id"),
        },
        _ => not_found("not found"),
    }
}

fn serve_image<R: Runtime>(app: &AppHandle<R>, id: i64) -> Response<Vec<u8>> {
    let state = app.state::<AppState>();
    let workspace = match state.workspace.lock() {
        Ok(workspace) => workspace,
        Err(_) => return internal_error("workspace lock poisoned"),
    };
    let Some(workspace) = workspace.as_ref() else {
        return not_found("no workspace");
    };

    let rel_path: String = match workspace.db.query_row(
        "SELECT rel_path FROM image WHERE id = ?1 AND deleted_at IS NULL",
        [id],
        |row| row.get(0),
    ) {
        Ok(path) => path,
        Err(_) => return not_found("image not found"),
    };

    let Some(path) = resolve_under_root(&workspace.root, &rel_path) else {
        return forbidden("path outside workspace");
    };
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(_) => return not_found("image file missing"),
    };
    response(StatusCode::OK, mime_for(&path), bytes)
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

fn response(status: StatusCode, mime: &str, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header("Content-Type", mime)
        .header("Cache-Control", "no-cache")
        .body(body)
        .expect("valid response")
}

fn not_found(message: &str) -> Response<Vec<u8>> {
    response(
        StatusCode::NOT_FOUND,
        "text/plain; charset=utf-8",
        message.as_bytes().to_vec(),
    )
}

fn forbidden(message: &str) -> Response<Vec<u8>> {
    response(
        StatusCode::FORBIDDEN,
        "text/plain; charset=utf-8",
        message.as_bytes().to_vec(),
    )
}

fn internal_error(message: &str) -> Response<Vec<u8>> {
    response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "text/plain; charset=utf-8",
        message.as_bytes().to_vec(),
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
