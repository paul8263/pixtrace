use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, UNIX_EPOCH};

use anyhow::{Context, Result};
use filetime::FileTime;
use image::codecs::webp::WebPEncoder;
use image::{ExtendedColorType, ImageReader};
use serde::Serialize;
use walkdir::WalkDir;

pub const GRID_THUMB_SIZE: u32 = 320;
pub const MAX_THUMB_SIZE: u32 = 1024;
const ENFORCE_INTERVAL: Duration = Duration::from_secs(60);

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
static LAST_ENFORCE: Mutex<Option<Instant>> = Mutex::new(None);

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub thumb_count: u64,
    pub thumb_bytes: u64,
    pub db_bytes: u64,
    pub limit_bytes: u64,
}

pub fn cache_dir(root: &Path) -> PathBuf {
    root.join(".pixtrace").join("cache").join("thumbs")
}

pub fn thumb_path(
    root: &Path,
    rel_path: &str,
    file_size: i64,
    mtime_ns: i64,
    size: u32,
) -> PathBuf {
    let key = blake3::hash(format!("{rel_path}|{file_size}|{mtime_ns}|{size}").as_bytes())
        .to_hex()
        .to_string();
    let prefix = &key[..2];
    cache_dir(root).join(prefix).join(format!("{key}.webp"))
}

pub fn ensure_thumb(
    root: &Path,
    source: &Path,
    rel_path: &str,
    file_size: i64,
    mtime_ns: i64,
    size: u32,
    limit_bytes: u64,
) -> Result<PathBuf> {
    let size = size.clamp(64, MAX_THUMB_SIZE);
    let cache_path = thumb_path(root, rel_path, file_size, mtime_ns, size);
    if cache_path.exists() {
        touch(&cache_path);
        return Ok(cache_path);
    }

    let image = ImageReader::open(source)
        .with_context(|| format!("打开图像失败: {}", source.display()))?
        .with_guessed_format()?
        .decode()
        .with_context(|| format!("解码图像失败: {}", source.display()))?;
    let thumbnail = image.thumbnail(size, size).to_rgba8();
    let (width, height) = thumbnail.dimensions();

    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建缩略图目录失败: {}", parent.display()))?;
    }

    let temp_path = cache_path.with_file_name(format!(
        ".{}.{}.{}.tmp",
        cache_path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("thumb"),
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    {
        let file = File::create(&temp_path)
            .with_context(|| format!("创建缩略图失败: {}", temp_path.display()))?;
        let mut writer = BufWriter::new(file);
        let encoder = WebPEncoder::new_lossless(&mut writer);
        encoder
            .encode(thumbnail.as_raw(), width, height, ExtendedColorType::Rgba8)
            .context("编码 WebP 缩略图失败")?;
        writer.flush()?;
    }

    if let Err(error) = fs::rename(&temp_path, &cache_path) {
        let _ = fs::remove_file(&temp_path);
        if !cache_path.exists() {
            return Err(error).context("保存缩略图失败");
        }
    }

    maybe_enforce_limit(root, limit_bytes);
    Ok(cache_path)
}

pub fn cache_stats(root: &Path, db_path: &Path, limit_bytes: u64) -> Result<CacheStats> {
    let mut thumb_count = 0u64;
    let mut thumb_bytes = 0u64;
    let dir = cache_dir(root);
    if dir.exists() {
        for entry in WalkDir::new(&dir).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            thumb_count += 1;
            thumb_bytes = thumb_bytes.saturating_add(metadata.len());
        }
    }
    let db_bytes = fs::metadata(db_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    Ok(CacheStats {
        thumb_count,
        thumb_bytes,
        db_bytes,
        limit_bytes,
    })
}

pub fn clear_cache(root: &Path) -> Result<()> {
    let dir = cache_dir(root);
    if dir.exists() {
        fs::remove_dir_all(&dir)
            .with_context(|| format!("清理缩略图缓存失败: {}", dir.display()))?;
    }
    Ok(())
}

pub fn enforce_limit(root: &Path, limit_bytes: u64) -> Result<u64> {
    let dir = cache_dir(root);
    if !dir.exists() {
        return Ok(0);
    }
    let mut files: Vec<(PathBuf, u64, std::time::SystemTime)> = Vec::new();
    let mut total = 0u64;
    for entry in WalkDir::new(&dir).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let size = metadata.len();
        let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
        total = total.saturating_add(size);
        files.push((entry.into_path(), size, modified));
    }
    if total <= limit_bytes {
        return Ok(0);
    }
    files.sort_by_key(|(_, _, modified)| *modified);
    let mut evicted = 0u64;
    for (path, size, _) in files {
        if total <= limit_bytes {
            break;
        }
        if fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(size);
            evicted += 1;
        }
    }
    Ok(evicted)
}

fn maybe_enforce_limit(root: &Path, limit_bytes: u64) {
    let now = Instant::now();
    if let Ok(mut last) = LAST_ENFORCE.lock() {
        if last.is_some_and(|value| now.duration_since(value) < ENFORCE_INTERVAL) {
            return;
        }
        *last = Some(now);
    }
    if let Err(error) = enforce_limit(root, limit_bytes) {
        tracing::warn!(%error, "thumbnail cache eviction failed");
    }
}

fn touch(path: &Path) {
    let _ = filetime::set_file_mtime(path, FileTime::now());
}
