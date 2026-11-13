use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GroupSort {
    #[default]
    Count,
    Recent,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupParams {
    #[serde(default)]
    pub sort: GroupSort,
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    100
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptGroup {
    pub prompt_hash: String,
    pub count: u64,
    pub cover_id: i64,
    pub sample: String,
    pub latest_mtime_ns: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupPage {
    pub items: Vec<PromptGroup>,
    pub total: u64,
}

pub fn list(conn: &Connection, params: &GroupParams) -> Result<GroupPage> {
    let order = match params.sort {
        GroupSort::Count => "n DESC, latest DESC, prompt_hash ASC",
        GroupSort::Recent => "latest DESC, n DESC, prompt_hash ASC",
    };
    let sql = format!(
        "SELECT prompt_hash, COUNT(*) AS n, MIN(id) AS cover_id,
                MIN(positive_prompt) AS sample, MAX(mtime_ns) AS latest
         FROM image
         WHERE deleted_at IS NULL AND prompt_hash IS NOT NULL AND prompt_hash <> ''
         GROUP BY prompt_hash
         ORDER BY {order}
         LIMIT ?1 OFFSET ?2"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(
        params![
            i64::from(params.limit.clamp(1, 500)),
            i64::from(params.offset)
        ],
        |row| {
            let count: i64 = row.get(1)?;
            let sample: Option<String> = row.get(3)?;
            Ok(PromptGroup {
                prompt_hash: row.get(0)?,
                count: count.max(0) as u64,
                cover_id: row.get(2)?,
                sample: sample.unwrap_or_default(),
                latest_mtime_ns: row.get(4)?,
            })
        },
    )?;
    let items = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM (
           SELECT 1 FROM image
           WHERE deleted_at IS NULL AND prompt_hash IS NOT NULL AND prompt_hash <> ''
           GROUP BY prompt_hash
         )",
        [],
        |row| row.get(0),
    )?;
    Ok(GroupPage {
        items,
        total: total.max(0) as u64,
    })
}
