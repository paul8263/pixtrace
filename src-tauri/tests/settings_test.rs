use pixtrace_lib::settings::{load, save, Settings, DEFAULT_THUMB_CACHE_LIMIT_MB};

#[test]
fn round_trips_settings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");

    assert!(load(&path).last_workspace.is_none());

    let settings = Settings {
        last_workspace: Some("C:\\outputs".to_string()),
        thumb_cache_limit_mb: 4096,
        watch_workspace: false,
    };
    save(&path, &settings).unwrap();

    let loaded = load(&path);
    assert_eq!(loaded.last_workspace.as_deref(), Some("C:\\outputs"));
    assert_eq!(loaded.thumb_cache_limit_mb, 4096);
    assert!(!loaded.watch_workspace);
}

#[test]
fn old_settings_receive_new_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(&path, r#"{"last_workspace":"C:\\outputs"}"#).unwrap();

    let loaded = load(&path);
    assert_eq!(loaded.last_workspace.as_deref(), Some("C:\\outputs"));
    assert_eq!(loaded.thumb_cache_limit_mb, DEFAULT_THUMB_CACHE_LIMIT_MB);
    assert!(loaded.watch_workspace);
}
