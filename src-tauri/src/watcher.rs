use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::AppHandle;

const DEBOUNCE: Duration = Duration::from_secs(2);

pub struct WorkspaceWatcher {
    watcher: Mutex<Option<RecommendedWatcher>>,
    cancel: Mutex<Option<Arc<AtomicBool>>>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl Default for WorkspaceWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceWatcher {
    pub fn new() -> Self {
        Self {
            watcher: Mutex::new(None),
            cancel: Mutex::new(None),
            handle: Mutex::new(None),
        }
    }

    pub fn start(&self, app: AppHandle, root: PathBuf) -> Result<()> {
        self.stop();

        let filter_root = root.clone();
        let (sender, receiver) = mpsc::sync_channel::<()>(1);
        let handler = move |result: notify::Result<Event>| {
            let Ok(event) = result else {
                return;
            };
            if event
                .paths
                .iter()
                .any(|path| is_watched_path(&filter_root, path))
            {
                let _ = sender.try_send(());
            }
        };
        let mut watcher =
            RecommendedWatcher::new(handler, Config::default()).context("创建文件监听器失败")?;
        watcher
            .watch(&root, RecursiveMode::Recursive)
            .with_context(|| format!("监听工作目录失败: {}", root.display()))?;

        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let worker_app = app.clone();
        let handle = std::thread::Builder::new()
            .name("pixtrace-watcher".to_string())
            .spawn(move || {
                let mut pending = false;
                let mut last_event = Instant::now();
                loop {
                    if worker_cancel.load(Ordering::SeqCst) {
                        break;
                    }
                    match receiver.recv_timeout(Duration::from_millis(200)) {
                        Ok(()) => {
                            pending = true;
                            last_event = Instant::now();
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            if pending && last_event.elapsed() >= DEBOUNCE {
                                pending = false;
                                if let Err(error) = crate::commands::rescan_current(&worker_app) {
                                    tracing::warn!(%error, "watcher rescan failed");
                                }
                            }
                        }
                        Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            })
            .context("启动文件监听线程失败")?;

        *self.watcher.lock().expect("watcher lock") = Some(watcher);
        *self.cancel.lock().expect("watcher cancel lock") = Some(cancel);
        *self.handle.lock().expect("watcher handle lock") = Some(handle);
        tracing::info!(root = %root.display(), "workspace watcher started");
        Ok(())
    }

    pub fn stop(&self) {
        if let Some(cancel) = self.cancel.lock().expect("watcher cancel lock").take() {
            cancel.store(true, Ordering::SeqCst);
        }
        self.watcher.lock().expect("watcher lock").take();
        if let Some(handle) = self.handle.lock().expect("watcher handle lock").take() {
            let _ = handle.join();
        }
    }
}

pub fn is_watched_path(root: &Path, path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };
    if !matches!(
        extension.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "webp"
    ) {
        return false;
    }

    let root_text = normalize_text(root);
    let path_text = normalize_text(path);
    let root_text = root_text.trim_end_matches(['\\', '/']);
    if root_text.is_empty() {
        return false;
    }
    let Some(head) = path_text.get(..root_text.len()) else {
        return false;
    };
    if !head.eq_ignore_ascii_case(root_text) {
        return false;
    }
    let rest = &path_text[root_text.len()..];
    if !rest.starts_with(['\\', '/']) {
        return false;
    }
    let relative = rest.trim_start_matches(['\\', '/']);
    !relative
        .split(['\\', '/'])
        .any(|segment| segment == ".pixtrace")
}

fn normalize_text(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.strip_prefix(r"\\?\")
        .or_else(|| text.strip_prefix(r"//?/"))
        .unwrap_or(&text)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::is_watched_path;
    use std::path::Path;

    #[test]
    fn watches_supported_images_inside_root() {
        let root = Path::new(r"C:\workspace");
        assert!(is_watched_path(root, Path::new(r"C:\workspace\a.png")));
        assert!(is_watched_path(
            root,
            Path::new(r"C:\workspace\nested\b.JPG")
        ));
        assert!(is_watched_path(
            root,
            Path::new(r"C:\workspace\nested\c.webp")
        ));
    }

    #[test]
    fn ignores_internal_cache_and_other_extensions() {
        let root = Path::new(r"C:\workspace");
        assert!(!is_watched_path(
            root,
            Path::new(r"C:\workspace\.pixtrace\cache\a.webp")
        ));
        assert!(!is_watched_path(root, Path::new(r"C:\workspace\a.txt")));
        assert!(!is_watched_path(root, Path::new(r"C:\workspace2\a.png")));
    }
}
