use pixtrace_lib::index::db;
use pixtrace_lib::scanner;

#[test]
fn scans_supported_images_and_skips_internal_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".pixtrace")).unwrap();
    std::fs::create_dir_all(dir.path().join("nested")).unwrap();
    std::fs::write(dir.path().join("a.png"), b"x").unwrap();
    std::fs::write(dir.path().join("nested").join("b.JPG"), b"x").unwrap();
    std::fs::write(dir.path().join("c.txt"), b"x").unwrap();
    std::fs::write(dir.path().join(".pixtrace").join("d.png"), b"x").unwrap();

    let db_path = dir.path().join(".pixtrace").join("index.db");
    let mut conn = db::open(&db_path).unwrap();
    let summary = scanner::scan(dir.path(), &mut conn).unwrap();

    assert_eq!(summary.image_count, 2);
    assert_eq!(summary.changed, 2);

    let mut statement = conn
        .prepare("SELECT rel_path FROM image ORDER BY rel_path")
        .unwrap();
    let paths: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(paths, vec!["a.png".to_string(), "nested/b.JPG".to_string()]);
}

#[test]
fn second_scan_reports_no_changes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.png"), b"x").unwrap();
    std::fs::write(dir.path().join("b.webp"), b"x").unwrap();

    let db_path = dir.path().join(".pixtrace").join("index.db");
    std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
    let mut conn = db::open(&db_path).unwrap();

    let first = scanner::scan(dir.path(), &mut conn).unwrap();
    assert_eq!(first.image_count, 2);
    assert_eq!(first.changed, 2);

    let second = scanner::scan(dir.path(), &mut conn).unwrap();
    assert_eq!(second.image_count, 2);
    assert_eq!(second.changed, 0);
}

#[test]
fn marks_missing_files_deleted_and_restores_them() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.png"), b"x").unwrap();
    std::fs::write(dir.path().join("b.png"), b"y").unwrap();

    let db_path = dir.path().join(".pixtrace").join("index.db");
    std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
    let mut conn = db::open(&db_path).unwrap();
    let first = scanner::scan(dir.path(), &mut conn).unwrap();
    assert_eq!(first.image_count, 2);

    std::fs::remove_file(dir.path().join("a.png")).unwrap();
    let second = scanner::scan(dir.path(), &mut conn).unwrap();
    assert_eq!(second.image_count, 1);
    assert_eq!(second.deleted, 1);

    let deleted_at: Option<i64> = conn
        .query_row(
            "SELECT deleted_at FROM image WHERE rel_path = 'a.png'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(deleted_at.is_some());

    std::fs::write(dir.path().join("a.png"), b"restored").unwrap();
    let third = scanner::scan(dir.path(), &mut conn).unwrap();
    assert_eq!(third.image_count, 2);
    let status: String = conn
        .query_row(
            "SELECT parse_status FROM image WHERE rel_path = 'a.png'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "pending");
}

#[test]
fn modified_files_are_marked_pending_again() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.png"), b"x").unwrap();
    let db_path = dir.path().join(".pixtrace").join("index.db");
    std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
    let mut conn = db::open(&db_path).unwrap();
    scanner::scan(dir.path(), &mut conn).unwrap();
    conn.execute("UPDATE image SET parse_status = 'ok'", [])
        .unwrap();

    std::fs::write(dir.path().join("a.png"), b"changed content").unwrap();
    let second = scanner::scan(dir.path(), &mut conn).unwrap();
    assert_eq!(second.changed, 1);
    let status: String = conn
        .query_row(
            "SELECT parse_status FROM image WHERE rel_path = 'a.png'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "pending");
}
