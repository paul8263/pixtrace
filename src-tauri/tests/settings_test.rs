use pixtrace_lib::settings::{load, save, Settings};

#[test]
fn round_trips_settings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");

    assert!(load(&path).last_workspace.is_none());

    let settings = Settings {
        last_workspace: Some("C:\\outputs".to_string()),
    };
    save(&path, &settings).unwrap();

    let loaded = load(&path);
    assert_eq!(loaded.last_workspace.as_deref(), Some("C:\\outputs"));
}
