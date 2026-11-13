use pixtrace_lib::index::{db, writer};
use pixtrace_lib::metadata;
use pixtrace_lib::prompt;
use pixtrace_lib::scanner;

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 12);
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&[0, 0, 0, 0]);
    out
}

fn build_png(prompt_json: &str) -> Vec<u8> {
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&512u32.to_be_bytes());
    ihdr.extend_from_slice(&768u32.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);

    let mut text = Vec::new();
    text.extend_from_slice(b"prompt");
    text.push(0);
    text.extend_from_slice(prompt_json.as_bytes());

    let mut bytes = vec![137, 80, 78, 71, 13, 10, 26, 10];
    bytes.extend_from_slice(&chunk(b"IHDR", &ihdr));
    bytes.extend_from_slice(&chunk(b"tEXt", &text));
    bytes.extend_from_slice(&chunk(b"IEND", &[]));
    bytes
}

#[test]
fn parses_and_writes_image_index_rows() {
    let dir = tempfile::tempdir().unwrap();
    let prompt_json = r#"{
      "3": {"class_type": "KSampler", "inputs": {
        "seed": 99, "steps": 24, "cfg": 6.5, "sampler_name": "euler",
        "positive": ["6", 0], "negative": ["7", 0], "latent_image": ["5", 0]}},
      "4": {"class_type": "CheckpointLoaderSimple", "inputs": {"ckpt_name": "model.safetensors"}},
      "5": {"class_type": "EmptyLatentImage", "inputs": {"width": 512, "height": 768}},
      "6": {"class_type": "CLIPTextEncode", "inputs": {"text": "masterpiece, 1girl"}},
      "7": {"class_type": "CLIPTextEncode", "inputs": {"text": "lowres"}}
    }"#;
    std::fs::write(dir.path().join("image.png"), build_png(prompt_json)).unwrap();

    let db_path = dir.path().join(".pixtrace").join("index.db");
    std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
    let mut conn = db::open(&db_path).unwrap();
    let summary = scanner::scan(dir.path(), &mut conn).unwrap();
    assert_eq!(summary.image_count, 1);

    let (id, rel_path, format): (i64, String, String) = conn
        .query_row("SELECT id, rel_path, format FROM image", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap();
    let meta = metadata::parse(&dir.path().join(&rel_path), &format).unwrap();
    let extracted = prompt::extract(&meta);
    assert_eq!(extracted.positive.as_deref(), Some("masterpiece, 1girl"));
    assert_eq!(extracted.negative.as_deref(), Some("lowres"));

    let parsed = writer::ParsedImage {
        id,
        width: extracted.width,
        height: extracted.height,
        status: extracted.status.as_str(),
        parse_error: extracted.error.clone(),
        extracted,
        prompt_json: meta.text("prompt").map(writer::compress),
        workflow_json: None,
        raw_text: None,
        indexed_at: 1,
    };
    writer::write_batch(&mut conn, &[parsed]).unwrap();

    let (status, positive, model, steps, fts): (String, String, String, i64, i64) = conn
        .query_row(
            "SELECT i.parse_status, i.positive_prompt, i.model_name, i.steps,
                    (SELECT COUNT(*) FROM image_fts WHERE image_fts MATCH 'masterpiece')
             FROM image i WHERE i.id = ?1",
            [id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(status, "ok");
    assert_eq!(positive, "masterpiece, 1girl");
    assert_eq!(model, "model.safetensors");
    assert_eq!(steps, 24);
    assert_eq!(fts, 1);

    let asset: Vec<u8> = conn
        .query_row(
            "SELECT prompt_json FROM image_asset WHERE image_id = ?1",
            [id],
            |row| row.get(0),
        )
        .unwrap();
    let json = String::from_utf8(writer::decompress(&asset).unwrap()).unwrap();
    assert!(json.contains("KSampler"));
}
