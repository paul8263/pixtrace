use pixtrace_lib::index::db;

#[test]
fn creates_schema_and_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.db");

    let conn = db::open(&path).unwrap();
    assert_eq!(db::schema_version(&conn).unwrap(), db::SCHEMA_VERSION);
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM image", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
    drop(conn);

    let conn = db::open(&path).unwrap();
    assert_eq!(db::schema_version(&conn).unwrap(), db::SCHEMA_VERSION);
}
