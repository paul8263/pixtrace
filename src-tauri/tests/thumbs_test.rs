use std::path::{Path, PathBuf};

use filetime::FileTime;
use pixtrace_lib::thumbs;

const LIMIT: u64 = 2 * 1024 * 1024 * 1024;

fn write_source(dir: &Path) -> PathBuf {
    let image = image::RgbaImage::from_fn(64, 32, |x, y| image::Rgba([x as u8, y as u8, 128, 255]));
    let path = dir.join("source.png");
    image::DynamicImage::ImageRgba8(image).save(&path).unwrap();
    path
}

#[test]
fn generates_and_caches_webp_thumbnail() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_source(dir.path());

    let cache =
        thumbs::ensure_thumb(dir.path(), &source, "source.png", 100, 200, 320, LIMIT).unwrap();
    assert!(cache.exists());
    let bytes = std::fs::read(&cache).unwrap();
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WEBP");

    let again =
        thumbs::ensure_thumb(dir.path(), &source, "source.png", 100, 200, 320, LIMIT).unwrap();
    assert_eq!(cache, again);
}

#[test]
fn cache_key_changes_with_mtime_and_size() {
    let dir = tempfile::tempdir().unwrap();
    let first = thumbs::thumb_path(dir.path(), "a.png", 100, 200, 320);
    let second = thumbs::thumb_path(dir.path(), "a.png", 100, 201, 320);
    let third = thumbs::thumb_path(dir.path(), "a.png", 101, 200, 320);
    assert_ne!(first, second);
    assert_ne!(first, third);
}

#[test]
fn evicts_oldest_thumbnails_over_limit() {
    let dir = tempfile::tempdir().unwrap();
    let cache_dir = thumbs::cache_dir(dir.path());
    std::fs::create_dir_all(&cache_dir).unwrap();
    for (index, name) in ["old.webp", "mid.webp", "new.webp"].iter().enumerate() {
        let path = cache_dir.join(name);
        std::fs::write(&path, vec![0u8; 10]).unwrap();
        filetime::set_file_mtime(
            &path,
            FileTime::from_unix_time(1_700_000_000 + index as i64, 0),
        )
        .unwrap();
    }

    let evicted = thumbs::enforce_limit(dir.path(), 20).unwrap();
    assert_eq!(evicted, 1);
    assert!(!cache_dir.join("old.webp").exists());
    assert!(cache_dir.join("mid.webp").exists());
    assert!(cache_dir.join("new.webp").exists());
}

#[test]
fn reports_and_clears_cache_stats() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("index.db");
    std::fs::write(&db_path, vec![0u8; 5]).unwrap();
    let cache_dir = thumbs::cache_dir(dir.path());
    std::fs::create_dir_all(&cache_dir).unwrap();
    std::fs::write(cache_dir.join("a.webp"), vec![0u8; 10]).unwrap();
    std::fs::write(cache_dir.join("b.webp"), vec![0u8; 10]).unwrap();

    let stats = thumbs::cache_stats(dir.path(), &db_path, 100).unwrap();
    assert_eq!(stats.thumb_count, 2);
    assert_eq!(stats.thumb_bytes, 20);
    assert_eq!(stats.db_bytes, 5);
    assert_eq!(stats.limit_bytes, 100);

    thumbs::clear_cache(dir.path()).unwrap();
    let stats = thumbs::cache_stats(dir.path(), &db_path, 100).unwrap();
    assert_eq!(stats.thumb_count, 0);
    assert_eq!(stats.thumb_bytes, 0);
}
