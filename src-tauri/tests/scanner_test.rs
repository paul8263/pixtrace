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
