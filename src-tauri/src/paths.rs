use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Installed,
    Portable,
}

#[derive(Debug, Clone)]
pub struct Paths {
    pub mode: Mode,
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
    pub webview_data_dir: PathBuf,
    pub settings_file: PathBuf,
    pub protocol_base: &'static str,
    pub warning: Option<String>,
}

impl Paths {
    pub fn resolve() -> Self {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."));
        let env_portable = std::env::var("PIXTRACE_PORTABLE")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        Self::resolve_from(&exe_dir, &args, env_portable)
    }

    pub fn resolve_from(exe_dir: &Path, args: &[String], env_portable: bool) -> Self {
        let mode = detect_mode(exe_dir, args, env_portable);
        let (data_dir, warning) = match mode {
            Mode::Portable => {
                let dir = exe_dir.join("data");
                if is_writable(&dir) {
                    (dir, None)
                } else {
                    let fallback = std::env::temp_dir().join("PixTrace-portable");
                    let _ = std::fs::create_dir_all(&fallback);
                    (
                        fallback,
                        Some("便携版目录不可写，已回退到临时目录，设置不会持久保存。".to_string()),
                    )
                }
            }
            Mode::Installed => {
                let dir = dirs::data_dir()
                    .unwrap_or_else(|| exe_dir.to_path_buf())
                    .join("PixTrace");
                if is_writable(&dir) {
                    (dir, None)
                } else {
                    let fallback = std::env::temp_dir().join("PixTrace");
                    let _ = std::fs::create_dir_all(&fallback);
                    (
                        fallback,
                        Some("应用数据目录不可写，已回退到临时目录。".to_string()),
                    )
                }
            }
        };

        let log_dir = data_dir.join("logs");
        let webview_data_dir = data_dir.join("WebView2");
        let settings_file = data_dir.join("settings.json");
        for dir in [&data_dir, &log_dir, &webview_data_dir] {
            let _ = std::fs::create_dir_all(dir);
        }

        Self {
            mode,
            data_dir,
            log_dir,
            webview_data_dir,
            settings_file,
            protocol_base: protocol_base(),
            warning,
        }
    }
}

pub fn detect_mode(exe_dir: &Path, args: &[String], env_portable: bool) -> Mode {
    if args.iter().any(|arg| arg == "--installed") {
        return Mode::Installed;
    }
    if args.iter().any(|arg| arg == "--portable") || env_portable {
        return Mode::Portable;
    }
    if exe_dir.join("portable.flag").exists() {
        Mode::Portable
    } else {
        Mode::Installed
    }
}

fn is_writable(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".write-test");
    match std::fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn protocol_base() -> &'static str {
    #[cfg(any(target_os = "windows", target_os = "android"))]
    {
        "http://pixtrace.localhost"
    }
    #[cfg(not(any(target_os = "windows", target_os = "android")))]
    {
        "pixtrace://localhost"
    }
}
