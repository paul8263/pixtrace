use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

use anyhow::{bail, Context, Result};
use flate2::read::ZlibDecoder;

use super::{ImageMetadata, TextChunk, MAX_TEXT_CHUNK};

const SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

pub fn parse(path: &Path) -> Result<ImageMetadata> {
    let file = File::open(path).with_context(|| format!("打开 PNG 失败: {}", path.display()))?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);

    let mut signature = [0u8; 8];
    reader.read_exact(&mut signature)?;
    if signature != SIGNATURE {
        bail!("不是 PNG 文件");
    }

    let mut meta = ImageMetadata::default();
    loop {
        let mut header = [0u8; 8];
        match reader.read_exact(&mut header) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                meta.truncated = true;
                break;
            }
            Err(error) => return Err(error.into()),
        }

        let length = u32::from_be_bytes(header[0..4].try_into().expect("4 bytes")) as usize;
        let chunk_type = [header[4], header[5], header[6], header[7]];

        let is_end = chunk_type == *b"IEND";
        match &chunk_type {
            b"IHDR" => read_ihdr(&mut reader, length, &mut meta)?,
            b"tEXt" | b"zTXt" | b"iTXt" => {
                if length <= MAX_TEXT_CHUNK {
                    let mut data = vec![0u8; length];
                    if reader.read_exact(&mut data).is_err() {
                        meta.truncated = true;
                        break;
                    }
                    if let Some(chunk) = parse_text_chunk(&chunk_type, &data) {
                        meta.texts.push(chunk);
                    }
                } else if skip(&mut reader, length).is_err() {
                    meta.truncated = true;
                    break;
                }
            }
            _ => {
                if skip(&mut reader, length).is_err() {
                    meta.truncated = true;
                    break;
                }
            }
        }

        let mut crc = [0u8; 4];
        if reader.read_exact(&mut crc).is_err() {
            meta.truncated = true;
            break;
        }
        if is_end {
            break;
        }
    }

    Ok(meta)
}

fn read_ihdr<R: Read>(reader: &mut R, length: usize, meta: &mut ImageMetadata) -> Result<()> {
    if !(13..=1024).contains(&length) {
        bail!("IHDR 长度异常: {}", length);
    }
    let mut data = [0u8; 13];
    reader.read_exact(&mut data)?;
    meta.width = Some(u32::from_be_bytes(data[0..4].try_into().expect("4 bytes")));
    meta.height = Some(u32::from_be_bytes(data[4..8].try_into().expect("4 bytes")));
    skip(reader, length - 13)?;
    Ok(())
}

fn parse_text_chunk(chunk_type: &[u8; 4], data: &[u8]) -> Option<TextChunk> {
    match chunk_type {
        b"tEXt" => {
            let (keyword, value) = split_nul(data)?;
            Some(TextChunk {
                keyword: decode_text(keyword),
                value: value.to_vec(),
            })
        }
        b"zTXt" => {
            let (keyword, rest) = split_nul(data)?;
            if rest.is_empty() || rest[0] != 0 {
                return None;
            }
            let value = inflate(&rest[1..])?;
            Some(TextChunk {
                keyword: decode_text(keyword),
                value,
            })
        }
        b"iTXt" => parse_itxt(data),
        _ => None,
    }
}

fn parse_itxt(data: &[u8]) -> Option<TextChunk> {
    let (keyword, rest) = split_nul(data)?;
    if rest.len() < 2 {
        return None;
    }
    let compressed = rest[0] == 1;
    if rest[1] != 0 {
        return None;
    }
    let rest = &rest[2..];
    let (_, rest) = split_nul(rest)?;
    let (_, text) = split_nul(rest)?;
    let value = if compressed {
        inflate(text)?
    } else {
        text.to_vec()
    };
    Some(TextChunk {
        keyword: decode_text(keyword),
        value,
    })
}

fn split_nul(data: &[u8]) -> Option<(&[u8], &[u8])> {
    let index = data.iter().position(|&byte| byte == 0)?;
    Some((&data[..index], &data[index + 1..]))
}

fn inflate(data: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data).take(MAX_TEXT_CHUNK as u64);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output).ok()?;
    Some(output)
}

fn decode_text(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => bytes.iter().map(|&byte| byte as char).collect(),
    }
}

fn skip<R: Read>(reader: &mut R, length: usize) -> io::Result<()> {
    let skipped = io::copy(&mut reader.take(length as u64), &mut io::sink())?;
    if skipped == length as u64 {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "PNG 数据被截断",
        ))
    }
}
