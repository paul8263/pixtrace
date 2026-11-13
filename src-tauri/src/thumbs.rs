use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use image::codecs::webp::WebPEncoder;
use image::{ExtendedColorType, ImageReader};

pub const GRID_THUMB_SIZE: u32 = 320;
pub const MAX_THUMB_SIZE: u32 = 1024;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

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
    root.join(".pixtrace")
        .join("cache")
        .join("thumbs")
        .join(prefix)
        .join(format!("{key}.webp"))
}

pub fn ensure_thumb(
    root: &Path,
    source: &Path,
    rel_path: &str,
    file_size: i64,
    mtime_ns: i64,
    size: u32,
) -> Result<PathBuf> {
    let size = size.clamp(64, MAX_THUMB_SIZE);
    let cache_path = thumb_path(root, rel_path, file_size, mtime_ns, size);
    if cache_path.exists() {
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

    Ok(cache_path)
}
