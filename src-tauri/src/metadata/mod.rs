pub mod png;

use std::path::Path;

use anyhow::Result;

pub const MAX_TEXT_CHUNK: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct TextChunk {
    pub keyword: String,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct ImageMetadata {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub texts: Vec<TextChunk>,
    pub truncated: bool,
}

impl ImageMetadata {
    pub fn text(&self, keyword: &str) -> Option<&[u8]> {
        self.texts
            .iter()
            .find(|chunk| chunk.keyword == keyword)
            .map(|chunk| chunk.value.as_slice())
    }
}

pub fn parse(path: &Path, format: &str) -> Result<ImageMetadata> {
    match format {
        "png" => png::parse(path),
        _ => dimensions_only(path),
    }
}

fn dimensions_only(path: &Path) -> Result<ImageMetadata> {
    let dimensions = image::ImageReader::open(path)?
        .with_guessed_format()?
        .into_dimensions()?;
    Ok(ImageMetadata {
        width: Some(dimensions.0),
        height: Some(dimensions.1),
        ..ImageMetadata::default()
    })
}
