use std::path::{Path, PathBuf};

use pixtrace_lib::thumbs;

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

    let cache = thumbs::ensure_thumb(dir.path(), &source, "source.png", 100, 200, 320).unwrap();
    assert!(cache.exists());
    let bytes = std::fs::read(&cache).unwrap();
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WEBP");

    let again = thumbs::ensure_thumb(dir.path(), &source, "source.png", 100, 200, 320).unwrap();
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
