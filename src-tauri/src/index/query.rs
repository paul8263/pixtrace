use anyhow::Result;
use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection};
use serde::{Deserialize, Serialize};

use crate::prompt;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sort {
    #[default]
    Relevance,
    Mtime,
    Path,
    Rating,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchParams {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub sampler: Option<String>,
    #[serde(default)]
    pub has_workflow: bool,
    #[serde(default)]
    pub rating: Option<u8>,
    #[serde(default)]
    pub min_rating: Option<u8>,
    #[serde(default)]
    pub unrated: bool,
    #[serde(default)]
    pub prompt_hash: Option<String>,
    #[serde(default)]
    pub sort: Sort,
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            query: String::new(),
            model: None,
            sampler: None,
            has_workflow: false,
            rating: None,
            min_rating: None,
            unrated: false,
            prompt_hash: None,
            sort: Sort::Relevance,
            offset: 0,
            limit: 200,
        }
    }
}

fn default_limit() -> u32 {
    200
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageRow {
    pub id: i64,
    pub rel_path: String,
    pub file_size: i64,
    pub mtime_ns: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub parse_status: String,
    pub model_name: Option<String>,
    pub stars: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub items: Vec<ImageRow>,
    pub total: u64,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterOptions {
    pub models: Vec<String>,
    pub samplers: Vec<String>,
    pub unrated: u64,
    pub stars_exact: [u64; 5],
    pub stars_min: [u64; 5],
}

#[derive(Debug)]
struct QueryTerm {
    tokens: Vec<String>,
    phrase: bool,
    exclude: bool,
}

pub fn search(conn: &Connection, params: &SearchParams) -> Result<SearchResult> {
    let (match_expr, exclude_expr) = build_fts_query(&params.query);
    let has_fts = match_expr.is_some();
    let from = if has_fts {
        "FROM image i JOIN image_fts ON image_fts.rowid = i.id"
    } else {
        "FROM image i"
    };

    let mut values: Vec<Value> = Vec::new();
    let mut clauses = vec!["i.deleted_at IS NULL".to_string()];
    if let Some(expr) = &match_expr {
        clauses.push(format!(
            "image_fts MATCH {}",
            bind(&mut values, Value::Text(expr.clone()))
        ));
    }
    if let Some(expr) = &exclude_expr {
        clauses.push(format!(
            "i.id NOT IN (SELECT rowid FROM image_fts WHERE image_fts MATCH {})",
            bind(&mut values, Value::Text(expr.clone()))
        ));
    }
    if let Some(model) = non_empty(&params.model) {
        clauses.push(format!(
            "i.model_name = {}",
            bind(&mut values, Value::Text(model.to_string()))
        ));
    }
    if let Some(sampler) = non_empty(&params.sampler) {
        clauses.push(format!(
            "i.sampler = {}",
            bind(&mut values, Value::Text(sampler.to_string()))
        ));
    }
    if params.has_workflow {
        clauses.push(
            "EXISTS (SELECT 1 FROM image_asset a WHERE a.image_id = i.id AND a.workflow_json IS NOT NULL)"
                .to_string(),
        );
    }
    if let Some(rating) = params.rating.filter(|value| (1..=5).contains(value)) {
        clauses.push(format!(
            "(SELECT stars FROM image_rating WHERE rel_path = i.rel_path) = {}",
            bind(&mut values, Value::Integer(i64::from(rating)))
        ));
    }
    if let Some(min_rating) = params.min_rating.filter(|value| (1..=5).contains(value)) {
        clauses.push(format!(
            "(SELECT stars FROM image_rating WHERE rel_path = i.rel_path) >= {}",
            bind(&mut values, Value::Integer(i64::from(min_rating)))
        ));
    }
    if params.unrated {
        clauses.push(
            "NOT EXISTS (SELECT 1 FROM image_rating WHERE rel_path = i.rel_path)".to_string(),
        );
    }
    if let Some(hash) = non_empty(&params.prompt_hash) {
        clauses.push(format!(
            "i.prompt_hash = {}",
            bind(&mut values, Value::Text(hash.to_string()))
        ));
    }
    let where_sql = clauses.join(" AND ");

    let count_sql = format!("SELECT COUNT(*) {from} WHERE {where_sql}");
    let total: i64 = conn.query_row(&count_sql, params_from_iter(values.iter()), |row| {
        row.get(0)
    })?;

    let order_sql = match params.sort {
        Sort::Mtime => "i.mtime_ns DESC, i.rel_path ASC",
        Sort::Path => "i.rel_path ASC",
        Sort::Rating => {
            "(SELECT stars FROM image_rating WHERE rel_path = i.rel_path) DESC,
             i.mtime_ns DESC, i.rel_path ASC"
        }
        Sort::Relevance if has_fts => "bm25(image_fts) ASC, i.rel_path ASC",
        Sort::Relevance => "i.rel_path ASC",
    };

    let limit_placeholder = bind(
        &mut values,
        Value::Integer(i64::from(params.limit.clamp(1, 500))),
    );
    let offset_placeholder = bind(&mut values, Value::Integer(i64::from(params.offset)));
    let page_sql = format!(
        "SELECT i.id, i.rel_path, i.file_size, i.mtime_ns, i.width, i.height,
                i.parse_status, i.model_name,
                (SELECT stars FROM image_rating WHERE rel_path = i.rel_path) AS stars
         {from} WHERE {where_sql}
         ORDER BY {order_sql} LIMIT {limit_placeholder} OFFSET {offset_placeholder}"
    );

    let mut statement = conn.prepare(&page_sql)?;
    let rows = statement.query_map(params_from_iter(values.iter()), |row| {
        Ok(ImageRow {
            id: row.get(0)?,
            rel_path: row.get(1)?,
            file_size: row.get(2)?,
            mtime_ns: row.get(3)?,
            width: row.get(4)?,
            height: row.get(5)?,
            parse_status: row.get(6)?,
            model_name: row.get(7)?,
            stars: row.get(8)?,
        })
    })?;
    let items = rows.collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(SearchResult {
        items,
        total: total.max(0) as u64,
    })
}

pub fn filter_options(conn: &Connection) -> Result<FilterOptions> {
    let mut statement = conn.prepare(
        "SELECT
           COUNT(CASE WHEN r.stars IS NULL THEN 1 END),
           COUNT(CASE WHEN r.stars = 1 THEN 1 END),
           COUNT(CASE WHEN r.stars = 2 THEN 1 END),
           COUNT(CASE WHEN r.stars = 3 THEN 1 END),
           COUNT(CASE WHEN r.stars = 4 THEN 1 END),
           COUNT(CASE WHEN r.stars = 5 THEN 1 END)
         FROM image i
         LEFT JOIN image_rating r ON r.rel_path = i.rel_path
         WHERE i.deleted_at IS NULL",
    )?;
    let (unrated, s1, s2, s3, s4, s5): (i64, i64, i64, i64, i64, i64) =
        statement.query_row([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })?;
    let stars_exact = [
        s1.max(0) as u64,
        s2.max(0) as u64,
        s3.max(0) as u64,
        s4.max(0) as u64,
        s5.max(0) as u64,
    ];
    let mut stars_min = [0u64; 5];
    let mut running = 0u64;
    for index in (0..5).rev() {
        running += stars_exact[index];
        stars_min[index] = running;
    }
    Ok(FilterOptions {
        models: collect_column(conn, "model_name")?,
        samplers: collect_column(conn, "sampler")?,
        unrated: unrated.max(0) as u64,
        stars_exact,
        stars_min,
    })
}

fn collect_column(conn: &Connection, column: &str) -> Result<Vec<String>> {
    let sql = format!(
        "SELECT {column}, COUNT(*) AS n FROM image
         WHERE deleted_at IS NULL AND {column} IS NOT NULL AND {column} <> ''
         GROUP BY {column}
         ORDER BY n DESC, {column} ASC"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn non_empty(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|text| !text.is_empty())
}

fn bind(values: &mut Vec<Value>, value: Value) -> String {
    values.push(value);
    format!("?{}", values.len())
}

fn build_fts_query(input: &str) -> (Option<String>, Option<String>) {
    let terms = parse_terms(input);
    let mut positive = Vec::new();
    let mut negative = Vec::new();
    for term in terms {
        if term.exclude {
            negative.push(term_expr(&term));
        } else {
            positive.push(term_expr(&term));
        }
    }
    let match_expr = if positive.is_empty() {
        None
    } else {
        Some(positive.join(" AND "))
    };
    let exclude_expr = if negative.is_empty() {
        None
    } else {
        Some(negative.join(" OR "))
    };
    (match_expr, exclude_expr)
}

fn parse_terms(input: &str) -> Vec<QueryTerm> {
    let chars: Vec<char> = input.chars().collect();
    let mut terms = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
        if index >= chars.len() {
            break;
        }
        let mut exclude = false;
        if chars[index] == '-' {
            exclude = true;
            index += 1;
        }
        if index >= chars.len() {
            break;
        }
        if chars[index] == '"' {
            index += 1;
            let mut text = String::new();
            while index < chars.len() && chars[index] != '"' {
                text.push(chars[index]);
                index += 1;
            }
            if index < chars.len() {
                index += 1;
            }
            let tokens = prompt::search_tokens(&text);
            if !tokens.is_empty() {
                terms.push(QueryTerm {
                    tokens,
                    phrase: true,
                    exclude,
                });
            }
        } else {
            let mut text = String::new();
            while index < chars.len() && !chars[index].is_whitespace() {
                text.push(chars[index]);
                index += 1;
            }
            let tokens = prompt::search_tokens(&text);
            if !tokens.is_empty() {
                terms.push(QueryTerm {
                    tokens,
                    phrase: false,
                    exclude,
                });
            }
        }
    }
    terms
}

fn term_expr(term: &QueryTerm) -> String {
    if term.tokens.len() == 1 {
        quote(&term.tokens[0])
    } else if term.phrase {
        format!(
            "\"{}\"",
            term.tokens
                .iter()
                .map(|token| token.replace('"', "\"\""))
                .collect::<Vec<_>>()
                .join(" ")
        )
    } else {
        format!(
            "({})",
            term.tokens
                .iter()
                .map(|token| quote(token))
                .collect::<Vec<_>>()
                .join(" AND ")
        )
    }
}

fn quote(token: &str) -> String {
    format!("\"{}\"", token.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_and_query() {
        let (positive, negative) = build_fts_query("red dress");
        assert_eq!(positive.as_deref(), Some("\"red\" AND \"dress\""));
        assert!(negative.is_none());
    }

    #[test]
    fn builds_phrase_query() {
        let (positive, _) = build_fts_query("\"red dress\"");
        assert_eq!(positive.as_deref(), Some("\"red dress\""));
    }

    #[test]
    fn builds_exclusion() {
        let (positive, negative) = build_fts_query("red -blue");
        assert_eq!(positive.as_deref(), Some("\"red\""));
        assert_eq!(negative.as_deref(), Some("\"blue\""));
    }

    #[test]
    fn builds_only_exclusion() {
        let (positive, negative) = build_fts_query("-blue");
        assert!(positive.is_none());
        assert_eq!(negative.as_deref(), Some("\"blue\""));
    }

    #[test]
    fn ignores_punctuation_only() {
        let (positive, negative) = build_fts_query("!!! ---");
        assert!(positive.is_none());
        assert!(negative.is_none());
    }

    #[test]
    fn tokenizes_chinese_query() {
        let (positive, _) = build_fts_query("一个女孩");
        let positive = positive.expect("positive query");
        assert!(positive.contains('"'));
        assert!(positive.contains("女孩"));
    }
}
