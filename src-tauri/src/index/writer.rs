use std::io::{Read, Write};

use anyhow::Result;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use rusqlite::{params, Connection};

use crate::prompt::Extracted;

#[derive(Debug)]
pub struct ParsedImage {
    pub id: i64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub status: &'static str,
    pub parse_error: Option<String>,
    pub extracted: Extracted,
    pub prompt_json: Option<Vec<u8>>,
    pub workflow_json: Option<Vec<u8>>,
    pub raw_text: Option<Vec<u8>>,
    pub indexed_at: i64,
}

pub fn write_batch(conn: &mut Connection, items: &[ParsedImage]) -> Result<()> {
    let tx = conn.transaction()?;
    for item in items {
        let extracted = &item.extracted;
        let search_positive = extracted
            .positive
            .as_deref()
            .map(crate::prompt::search_text);
        let search_negative = extracted
            .negative
            .as_deref()
            .map(crate::prompt::search_text);
        let prompt_hash = extracted
            .positive
            .as_deref()
            .map(crate::prompt::normalize_prompt)
            .filter(|normalized| !normalized.is_empty())
            .map(|normalized| crate::prompt::prompt_hash(&normalized));
        let loras = extracted.loras_text();

        tx.execute(
            "UPDATE image SET
               width = ?2, height = ?3, parse_status = ?4, parse_error = ?5,
               positive_prompt = ?6, negative_prompt = ?7, prompt_hash = ?8,
               search_positive = ?9, search_negative = ?10, model_name = ?11,
               loras = ?12, sampler = ?13, steps = ?14, cfg = ?15, seed = ?16,
               indexed_at = ?17
             WHERE id = ?1",
            params![
                item.id,
                item.width,
                item.height,
                item.status,
                item.parse_error,
                extracted.positive,
                extracted.negative,
                prompt_hash,
                search_positive,
                search_negative,
                extracted.model,
                loras,
                extracted.sampler,
                extracted.steps,
                extracted.cfg,
                extracted.seed,
                item.indexed_at,
            ],
        )?;

        tx.execute("DELETE FROM image_fts WHERE rowid = ?1", params![item.id])?;
        tx.execute(
            "INSERT INTO image_fts(rowid, positive, negative, model, loras)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                item.id,
                search_positive,
                search_negative,
                extracted.model,
                loras,
            ],
        )?;

        tx.execute(
            "INSERT INTO image_asset(image_id, prompt_json, workflow_json, raw_text)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(image_id) DO UPDATE SET
               prompt_json = excluded.prompt_json,
               workflow_json = excluded.workflow_json,
               raw_text = excluded.raw_text",
            params![item.id, item.prompt_json, item.workflow_json, item.raw_text],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    if encoder.write_all(data).is_err() {
        return data.to_vec();
    }
    encoder.finish().unwrap_or_else(|_| data.to_vec())
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    Ok(output)
}
