use pixtrace_lib::paths::{detect_mode, Mode};

#[test]
fn defaults_to_installed() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(detect_mode(dir.path(), &[], false), Mode::Installed);
}

#[test]
fn portable_flag_enables_portable_mode() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("portable.flag"), b"").unwrap();
    assert_eq!(detect_mode(dir.path(), &[], false), Mode::Portable);
}

#[test]
fn cli_flags_override_flag_file() {
    let with_flag = tempfile::tempdir().unwrap();
    std::fs::write(with_flag.path().join("portable.flag"), b"").unwrap();
    assert_eq!(
        detect_mode(with_flag.path(), &["--installed".to_string()], false),
        Mode::Installed
    );

    let without_flag = tempfile::tempdir().unwrap();
    assert_eq!(
        detect_mode(without_flag.path(), &["--portable".to_string()], false),
        Mode::Portable
    );
}

#[test]
fn env_var_enables_portable_mode() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(detect_mode(dir.path(), &[], true), Mode::Portable);
}
