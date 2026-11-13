use pixtrace_lib::index::db;
use pixtrace_lib::ratings;
use pixtrace_lib::scanner;
use rusqlite::{params, Connection};

fn setup() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".pixtrace")).unwrap();
    let conn = db::open(&dir.path().join(".pixtrace").join("index.db")).unwrap();
    for (index, rel_path) in ["a.png", "b.png"].iter().enumerate() {
        conn.execute(
            "INSERT INTO image(rel_path, file_size, mtime_ns, format, parse_status)
             VALUES (?1, 1, ?2, 'png', 'ok')",
            params![rel_path, index as i64],
        )
        .unwrap();
    }
    (dir, conn)
}

fn image_id(conn: &Connection, rel_path: &str) -> i64 {
    conn.query_row(
        "SELECT id FROM image WHERE rel_path = ?1",
        [rel_path],
        |row| row.get(0),
    )
    .unwrap()
}

fn stars(conn: &Connection, rel_path: &str) -> Option<u8> {
    conn.query_row(
        "SELECT stars FROM image_rating WHERE rel_path = ?1",
        [rel_path],
        |row| row.get(0),
    )
    .ok()
}

#[test]
fn sets_and_clears_rating() {
    let (_dir, conn) = setup();
    let id = image_id(&conn, "a.png");

    let result = ratings::set(&conn, id, 5).unwrap();
    assert_eq!(result.rel_path, "a.png");
    assert_eq!(result.stars, Some(5));
    assert_eq!(stars(&conn, "a.png"), Some(5));

    ratings::set(&conn, id, 3).unwrap();
    assert_eq!(stars(&conn, "a.png"), Some(3));

    assert!(ratings::set(&conn, id, 0).is_err());
    assert!(ratings::set(&conn, id, 6).is_err());

    let cleared = ratings::clear(&conn, id).unwrap();
    assert_eq!(cleared.stars, None);
    assert_eq!(stars(&conn, "a.png"), None);
}

#[test]
fn exports_and_restores_snapshot_after_database_loss() {
    let (dir, conn) = setup();
    ratings::set(&conn, image_id(&conn, "a.png"), 5).unwrap();
    ratings::set(&conn, image_id(&conn, "b.png"), 2).unwrap();
    ratings::export(&conn, dir.path()).unwrap();

    let snapshot = std::fs::read_to_string(ratings::snapshot_path(dir.path())).unwrap();
    assert!(snapshot.contains("\"relPath\": \"a.png\""));
    assert!(snapshot.contains("\"stars\": 5"));

    drop(conn);
    for name in ["index.db", "index.db-wal", "index.db-shm"] {
        let _ = std::fs::remove_file(dir.path().join(".pixtrace").join(name));
    }

    let mut fresh = db::open(&dir.path().join(".pixtrace").join("index.db")).unwrap();
    let restored = ratings::restore(&mut fresh, dir.path()).unwrap();
    assert_eq!(restored, 2);
    assert_eq!(stars(&fresh, "a.png"), Some(5));
    assert_eq!(stars(&fresh, "b.png"), Some(2));
}

#[test]
fn restore_does_not_overwrite_newer_database_ratings() {
    let (dir, conn) = setup();
    ratings::set(&conn, image_id(&conn, "a.png"), 5).unwrap();
    ratings::export(&conn, dir.path()).unwrap();
    conn.execute(
        "UPDATE image_rating SET stars = 4, updated_at = 9999999999
         WHERE rel_path = 'a.png'",
        [],
    )
    .unwrap();

    let mut conn = conn;
    ratings::restore(&mut conn, dir.path()).unwrap();
    assert_eq!(stars(&conn, "a.png"), Some(4));
}

#[test]
fn snapshot_keeps_ratings_for_missing_files() {
    let (dir, conn) = setup();
    conn.execute(
        "INSERT INTO image_rating(rel_path, stars, updated_at)
         VALUES ('gone.png', 5, 100)",
        [],
    )
    .unwrap();
    ratings::export(&conn, dir.path()).unwrap();
    drop(conn);

    for name in ["index.db", "index.db-wal", "index.db-shm"] {
        let _ = std::fs::remove_file(dir.path().join(".pixtrace").join(name));
    }
    let mut fresh = db::open(&dir.path().join(".pixtrace").join("index.db")).unwrap();
    ratings::restore(&mut fresh, dir.path()).unwrap();
    assert_eq!(stars(&fresh, "gone.png"), Some(5));
}

#[test]
fn corrupt_snapshot_is_reported() {
    let (dir, conn) = setup();
    std::fs::write(ratings::snapshot_path(dir.path()), b"{not json").unwrap();
    let mut conn = conn;
    assert!(ratings::restore(&mut conn, dir.path()).is_err());
}

#[test]
fn rating_survives_file_deletion_and_restore() {
    let (dir, mut conn) = setup();
    std::fs::write(dir.path().join("a.png"), b"x").unwrap();
    std::fs::write(dir.path().join("b.png"), b"x").unwrap();
    let id = image_id(&conn, "a.png");
    ratings::set(&conn, id, 5).unwrap();

    std::fs::remove_file(dir.path().join("a.png")).unwrap();
    let deleted = scanner::scan(dir.path(), &mut conn).unwrap();
    assert_eq!(deleted.deleted, 1);
    assert_eq!(stars(&conn, "a.png"), Some(5));

    std::fs::write(dir.path().join("a.png"), b"restored").unwrap();
    let restored = scanner::scan(dir.path(), &mut conn).unwrap();
    assert_eq!(restored.image_count, 2);
    assert_eq!(stars(&conn, "a.png"), Some(5));
    let status: String = conn
        .query_row(
            "SELECT parse_status FROM image WHERE rel_path = 'a.png'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "pending");
}
