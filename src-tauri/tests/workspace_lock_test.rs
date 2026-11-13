use pixtrace_lib::commands::acquire_workspace_lock;

#[test]
fn acquires_lock_and_writes_owner_info() {
    let dir = tempfile::tempdir().unwrap();
    let pixtrace = dir.path().join(".pixtrace");
    let lock = acquire_workspace_lock(&pixtrace).unwrap();
    assert!(pixtrace.join("workspace.lock").exists());
    let text = std::fs::read_to_string(pixtrace.join("workspace.owner")).unwrap();
    assert!(text.contains(&format!("pid={}", std::process::id())));
    drop(lock);
}

#[test]
fn rejects_second_instance_for_same_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let pixtrace = dir.path().join(".pixtrace");
    let first = acquire_workspace_lock(&pixtrace).unwrap();
    let error = acquire_workspace_lock(&pixtrace).unwrap_err().to_string();
    assert!(error.contains("另一个 PixTrace 实例"), "{error}");
    drop(first);
    acquire_workspace_lock(&pixtrace).unwrap();
}
