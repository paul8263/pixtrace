use std::time::{Duration, Instant};

use pixtrace_lib::index::groups::{self, GroupParams, GroupSort};
use pixtrace_lib::index::query::{self, SearchParams, Sort};
use pixtrace_lib::index::{db, writer};
use pixtrace_lib::prompt;
use pixtrace_lib::prompt::{Extracted, ParseStatus};
use pixtrace_lib::ratings;
use rusqlite::{params, Connection};

fn make_image(
    conn: &mut Connection,
    rel_path: &str,
    mtime_ns: i64,
    extracted: Extracted,
    workflow: bool,
) -> i64 {
    conn.execute(
        "INSERT INTO image(rel_path, file_size, mtime_ns, format, parse_status)
         VALUES (?1, 100, ?2, 'png', 'pending')",
        params![rel_path, mtime_ns],
    )
    .unwrap();
    let id = conn.last_insert_rowid();
    let parsed = writer::ParsedImage {
        id,
        width: extracted.width,
        height: extracted.height,
        status: extracted.status.as_str(),
        parse_error: extracted.error.clone(),
        extracted,
        prompt_json: None,
        workflow_json: workflow.then(|| b"{}".to_vec()),
        raw_text: None,
        indexed_at: 1,
    };
    writer::write_batch(conn, &[parsed]).unwrap();
    id
}

fn sample_prompt(positive: &str, negative: &str) -> Extracted {
    Extracted {
        positive: Some(positive.to_string()),
        negative: Some(negative.to_string()),
        status: ParseStatus::Ok,
        ..Default::default()
    }
}

fn setup() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = db::open(&dir.path().join("index.db")).unwrap();

    let mut red = sample_prompt("masterpiece, red dress, 1girl", "lowres");
    red.model = Some("sd15.safetensors".to_string());
    red.sampler = Some("euler".to_string());
    make_image(&mut conn, "a-red.png", 10, red, false);

    let mut blue = sample_prompt("masterpiece, blue dress, 1girl", "blurry");
    blue.model = Some("sdxl.safetensors".to_string());
    blue.sampler = Some("dpmpp_2m".to_string());
    make_image(&mut conn, "b-blue.png", 30, blue, false);

    let mut chinese = sample_prompt("一个女孩站在花丛中，白色连衣裙", "低质量");
    chinese.model = Some("flux.safetensors".to_string());
    chinese.sampler = Some("euler".to_string());
    make_image(&mut conn, "c-cn.png", 20, chinese, true);

    (dir, conn)
}

fn run(conn: &Connection, text: &str) -> Vec<String> {
    query::search(
        conn,
        &SearchParams {
            query: text.to_string(),
            model: None,
            sampler: None,
            has_workflow: false,
            sort: Sort::Path,
            offset: 0,
            limit: 50,
            ..Default::default()
        },
    )
    .unwrap()
    .items
    .into_iter()
    .map(|item| item.rel_path)
    .collect()
}

fn image_id(conn: &Connection, rel_path: &str) -> i64 {
    conn.query_row(
        "SELECT id FROM image WHERE rel_path = ?1",
        [rel_path],
        |row| row.get(0),
    )
    .unwrap()
}

#[test]
fn searches_words_with_and_semantics() {
    let (_dir, conn) = setup();
    assert_eq!(run(&conn, "red"), vec!["a-red.png"]);
    assert_eq!(
        run(&conn, "masterpiece dress"),
        vec!["a-red.png", "b-blue.png"]
    );
    assert_eq!(
        run(&conn, "\"red dress\""),
        vec!["a-red.png"],
        "quoted phrase should match adjacent tokens"
    );
    assert_eq!(
        run(&conn, "masterpiece -blue"),
        vec!["a-red.png"],
        "excluded word should remove matching documents"
    );
}

#[test]
fn sorts_by_relevance() {
    let (_dir, conn) = setup();
    let result = query::search(
        &conn,
        &SearchParams {
            query: "masterpiece dress".to_string(),
            model: None,
            sampler: None,
            has_workflow: false,
            sort: Sort::Relevance,
            offset: 0,
            limit: 50,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.total, 2);
    let paths: Vec<_> = result.items.into_iter().map(|item| item.rel_path).collect();
    assert_eq!(paths, vec!["a-red.png", "b-blue.png"]);
}

#[test]
fn searches_negative_prompt_and_chinese() {
    let (_dir, conn) = setup();
    assert_eq!(run(&conn, "lowres"), vec!["a-red.png"]);
    assert_eq!(run(&conn, "女孩"), vec!["c-cn.png"]);
    assert_eq!(run(&conn, "连衣裙"), vec!["c-cn.png"]);
    assert_eq!(run(&conn, "白色连衣裙"), vec!["c-cn.png"]);
}

#[test]
fn supports_exclusion_only_query() {
    let (_dir, conn) = setup();
    assert_eq!(
        run(&conn, "-masterpiece"),
        vec!["c-cn.png"],
        "only-exclusion query should return all non-matching rows"
    );
    assert_eq!(
        run(&conn, "-dress"),
        vec!["c-cn.png"],
        "documents containing the excluded token are filtered"
    );
}

#[test]
fn applies_filters_and_sort() {
    let (_dir, conn) = setup();

    let model = query::search(
        &conn,
        &SearchParams {
            query: String::new(),
            model: Some("flux.safetensors".to_string()),
            sampler: None,
            has_workflow: false,
            sort: Sort::Path,
            offset: 0,
            limit: 50,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(model.total, 1);
    assert_eq!(model.items[0].rel_path, "c-cn.png");

    let workflow = query::search(
        &conn,
        &SearchParams {
            query: String::new(),
            model: None,
            sampler: Some("euler".to_string()),
            has_workflow: true,
            sort: Sort::Path,
            offset: 0,
            limit: 50,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(workflow.total, 1);
    assert_eq!(workflow.items[0].rel_path, "c-cn.png");

    let newest = query::search(
        &conn,
        &SearchParams {
            query: String::new(),
            model: None,
            sampler: None,
            has_workflow: false,
            sort: Sort::Mtime,
            offset: 0,
            limit: 50,
            ..Default::default()
        },
    )
    .unwrap();
    let paths: Vec<_> = newest.items.into_iter().map(|item| item.rel_path).collect();
    assert_eq!(paths, vec!["b-blue.png", "c-cn.png", "a-red.png"]);
}

#[test]
fn filters_and_sorts_by_rating() {
    let (_dir, conn) = setup();
    let a = image_id(&conn, "a-red.png");
    let b = image_id(&conn, "b-blue.png");
    ratings::set(&conn, a, 5).unwrap();
    ratings::set(&conn, b, 3).unwrap();

    let at_least_four = query::search(
        &conn,
        &SearchParams {
            min_rating: Some(4),
            sort: Sort::Rating,
            limit: 50,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(at_least_four.total, 1);
    assert_eq!(at_least_four.items[0].rel_path, "a-red.png");
    assert_eq!(at_least_four.items[0].stars, Some(5));

    let exactly_three = query::search(
        &conn,
        &SearchParams {
            rating: Some(3),
            limit: 50,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(exactly_three.total, 1);
    assert_eq!(exactly_three.items[0].rel_path, "b-blue.png");

    let unrated = query::search(
        &conn,
        &SearchParams {
            unrated: true,
            limit: 50,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(unrated.total, 1);
    assert_eq!(unrated.items[0].rel_path, "c-cn.png");
    assert_eq!(unrated.items[0].stars, None);

    let sorted = query::search(
        &conn,
        &SearchParams {
            sort: Sort::Rating,
            limit: 50,
            ..Default::default()
        },
    )
    .unwrap();
    let paths: Vec<_> = sorted.items.into_iter().map(|item| item.rel_path).collect();
    assert_eq!(paths, vec!["a-red.png", "b-blue.png", "c-cn.png"]);

    let options = query::filter_options(&conn).unwrap();
    assert_eq!(options.unrated, 1);
    assert_eq!(options.stars_exact, [0, 0, 1, 0, 1]);
    assert_eq!(options.stars_min, [2, 2, 2, 1, 1]);
}

#[test]
fn groups_images_by_normalized_prompt() {
    let (_dir, mut conn) = setup();
    make_image(
        &mut conn,
        "a-copy-old.png",
        5,
        sample_prompt("masterpiece, red dress, 1girl", "lowres"),
        false,
    );
    make_image(
        &mut conn,
        "a-copy-new.png",
        6,
        sample_prompt("masterpiece, red dress, 1girl", "lowres"),
        false,
    );

    let by_count = groups::list(
        &conn,
        &GroupParams {
            sort: GroupSort::Count,
            offset: 0,
            limit: 50,
        },
    )
    .unwrap();
    assert_eq!(by_count.total, 3);
    assert_eq!(by_count.items[0].count, 3);
    assert_eq!(by_count.items[0].sample, "masterpiece, red dress, 1girl");
    assert_eq!(by_count.items[0].cover_id, image_id(&conn, "a-red.png"));

    let by_recent = groups::list(
        &conn,
        &GroupParams {
            sort: GroupSort::Recent,
            offset: 0,
            limit: 50,
        },
    )
    .unwrap();
    assert_eq!(
        by_recent.items[0].prompt_hash,
        by_count.items[1].prompt_hash
    );
    assert_eq!(by_recent.items[0].sample, "masterpiece, blue dress, 1girl");

    let hash = by_count.items[0].prompt_hash.clone();
    let group_images = query::search(
        &conn,
        &SearchParams {
            prompt_hash: Some(hash),
            sort: Sort::Path,
            limit: 50,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(group_images.total, 3);
    let paths: Vec<_> = group_images
        .items
        .into_iter()
        .map(|item| item.rel_path)
        .collect();
    assert_eq!(paths, vec!["a-copy-new.png", "a-copy-old.png", "a-red.png"]);
}

#[test]
fn paginates_results() {
    let (_dir, conn) = setup();
    let first = query::search(
        &conn,
        &SearchParams {
            query: "masterpiece".to_string(),
            model: None,
            sampler: None,
            has_workflow: false,
            sort: Sort::Path,
            offset: 0,
            limit: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(first.total, 2);
    assert_eq!(first.items.len(), 1);
    assert_eq!(first.items[0].rel_path, "a-red.png");

    let second = query::search(
        &conn,
        &SearchParams {
            query: "masterpiece".to_string(),
            model: None,
            sampler: None,
            has_workflow: false,
            sort: Sort::Path,
            offset: 1,
            limit: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(second.total, 2);
    assert_eq!(second.items[0].rel_path, "b-blue.png");
}

#[test]
fn returns_filter_options() {
    let (_dir, conn) = setup();
    let options = query::filter_options(&conn).unwrap();
    assert_eq!(
        options.models,
        vec!["flux.safetensors", "sd15.safetensors", "sdxl.safetensors"]
    );
    assert_eq!(options.samplers, vec!["euler", "dpmpp_2m"]);
}

#[test]
#[ignore = "50k-row latency check; run with cargo test --release -- --ignored"]
fn search_latency_p95_is_under_150ms() {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = db::open(&dir.path().join("index.db")).unwrap();
    let tx = conn.transaction().unwrap();
    {
        let mut image_stmt = tx
            .prepare(
                "INSERT INTO image(rel_path, file_size, mtime_ns, format, parse_status,
                                   positive_prompt, search_positive, model_name, sampler)
                 VALUES (?1, 1000, ?2, 'png', 'ok', ?3, ?4, 'model.safetensors', 'euler')",
            )
            .unwrap();
        let mut fts_stmt = tx
            .prepare(
                "INSERT INTO image_fts(rowid, positive, negative, model, loras)
                 VALUES (?1, ?2, '', 'model.safetensors', '')",
            )
            .unwrap();
        for index in 0..50_000i64 {
            let positive = format!("masterpiece, 1girl, red dress, scene {}", index % 1000);
            let search = prompt::search_text(&positive);
            image_stmt
                .execute(params![
                    format!("img-{index:05}.png"),
                    index,
                    positive,
                    search
                ])
                .unwrap();
            let id = tx.last_insert_rowid();
            fts_stmt.execute(params![id, search]).unwrap();
        }
    }
    tx.commit().unwrap();

    let queries = [
        ("red dress", Sort::Relevance),
        ("masterpiece", Sort::Relevance),
        ("scene 123", Sort::Path),
        ("red -blue", Sort::Relevance),
        ("-blue", Sort::Path),
        ("masterpiece", Sort::Rating),
        ("", Sort::Rating),
    ];
    for (text, sort) in queries {
        query::search(
            &conn,
            &SearchParams {
                query: text.to_string(),
                model: None,
                sampler: None,
                has_workflow: false,
                sort,
                offset: 0,
                limit: 200,
                ..Default::default()
            },
        )
        .unwrap();
    }

    let mut timings = Vec::new();
    for _ in 0..20 {
        for (text, sort) in queries {
            let start = Instant::now();
            let result = query::search(
                &conn,
                &SearchParams {
                    query: text.to_string(),
                    model: None,
                    sampler: None,
                    has_workflow: false,
                    sort,
                    offset: 0,
                    limit: 200,
                    ..Default::default()
                },
            )
            .unwrap();
            assert!(result.total > 0);
            timings.push(start.elapsed());
        }
    }
    timings.sort();
    let p95 = timings[timings.len() * 95 / 100];
    println!("p95 search latency over 50k rows: {p95:?}");
    assert!(
        p95 < Duration::from_millis(150),
        "p95 search latency was {p95:?}"
    );
}
