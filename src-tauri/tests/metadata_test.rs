use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;
use pixtrace_lib::metadata::png;

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 12);
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&[0, 0, 0, 0]);
    out
}

fn ihdr(width: u32, height: u32) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&width.to_be_bytes());
    data.extend_from_slice(&height.to_be_bytes());
    data.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &data)
}

fn text(keyword: &str, value: &str) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(keyword.as_bytes());
    data.push(0);
    data.extend_from_slice(value.as_bytes());
    chunk(b"tEXt", &data)
}

fn compressed_text(keyword: &str, value: &str) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(value.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();
    let mut data = Vec::new();
    data.extend_from_slice(keyword.as_bytes());
    data.push(0);
    data.push(0);
    data.extend_from_slice(&compressed);
    chunk(b"zTXt", &data)
}

fn itxt(keyword: &str, value: &str, compressed: bool) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(keyword.as_bytes());
    data.push(0);
    data.push(if compressed { 1 } else { 0 });
    data.push(0);
    data.push(0);
    data.push(0);
    if compressed {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(value.as_bytes()).unwrap();
        data.extend_from_slice(&encoder.finish().unwrap());
    } else {
        data.extend_from_slice(value.as_bytes());
    }
    chunk(b"iTXt", &data)
}

fn write_png(parts: &[Vec<u8>]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let mut bytes = vec![137, 80, 78, 71, 13, 10, 26, 10];
    for part in parts {
        bytes.extend_from_slice(part);
    }
    std::fs::write(dir.path().join("test.png"), bytes).unwrap();
    dir
}

#[test]
fn reads_text_chunks_around_idat() {
    let dir = write_png(&[
        ihdr(640, 480),
        text("prompt", r#"{"1":{"class_type":"KSampler"}}"#),
        chunk(b"IDAT", &vec![0u8; 300_000]),
        compressed_text("workflow", r#"{"nodes":[]}"#),
        itxt("parameters", "Steps: 20", false),
        chunk(b"IEND", &[]),
    ]);

    let meta = png::parse(&dir.path().join("test.png")).unwrap();
    assert_eq!(meta.width, Some(640));
    assert_eq!(meta.height, Some(480));
    assert!(!meta.truncated);
    assert_eq!(
        meta.text("prompt").map(String::from_utf8_lossy),
        Some(r#"{"1":{"class_type":"KSampler"}}"#.into())
    );
    assert_eq!(
        meta.text("workflow").map(String::from_utf8_lossy),
        Some(r#"{"nodes":[]}"#.into())
    );
    assert_eq!(
        meta.text("parameters").map(String::from_utf8_lossy),
        Some("Steps: 20".into())
    );
}

#[test]
fn reads_utf8_itxt_text() {
    let dir = write_png(&[
        ihdr(512, 512),
        itxt("prompt", "一只在月光下的猫", false),
        chunk(b"IEND", &[]),
    ]);
    let meta = png::parse(&dir.path().join("test.png")).unwrap();
    assert_eq!(
        meta.text("prompt").map(String::from_utf8_lossy),
        Some("一只在月光下的猫".into())
    );
}

#[test]
fn reads_compressed_itxt() {
    let dir = write_png(&[
        ihdr(1, 1),
        itxt("prompt", "compressed text", true),
        chunk(b"IEND", &[]),
    ]);
    let meta = png::parse(&dir.path().join("test.png")).unwrap();
    assert_eq!(
        meta.text("prompt").map(String::from_utf8_lossy),
        Some("compressed text".into())
    );
}

#[test]
fn truncated_file_does_not_panic() {
    let mut bytes = vec![137, 80, 78, 71, 13, 10, 26, 10];
    bytes.extend_from_slice(&ihdr(10, 10));
    bytes.extend_from_slice(&text("prompt", "partial"));
    bytes.truncate(bytes.len() - 3);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("truncated.png");
    std::fs::write(&path, bytes).unwrap();

    let result = png::parse(&path);
    if let Ok(meta) = result {
        assert!(meta.truncated || meta.text("prompt").is_some());
    }
}

#[test]
fn rejects_non_png() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("not.png");
    std::fs::write(&path, b"hello").unwrap();
    assert!(png::parse(&path).is_err());
}

#[test]
fn ignores_corrupt_compressed_chunk() {
    let mut data = Vec::new();
    data.extend_from_slice(b"prompt");
    data.push(0);
    data.push(0);
    data.extend_from_slice(b"not zlib data");
    let dir = write_png(&[ihdr(2, 2), chunk(b"zTXt", &data), chunk(b"IEND", &[])]);
    let meta = png::parse(&dir.path().join("test.png")).unwrap();
    assert_eq!(meta.width, Some(2));
    assert!(meta.text("prompt").is_none());
}
