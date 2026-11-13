pub mod commands;
pub mod index;
pub mod indexer;
pub mod metadata;
pub mod paths;
pub mod prompt;
pub mod protocol;
pub mod ratings;
pub mod scanner;
pub mod settings;
pub mod thumbs;
pub mod watcher;

use commands::AppState;
use paths::{Mode, Paths};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn run() {
    let paths = Paths::resolve();
    if paths.mode == Mode::Portable {
        std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", &paths.webview_data_dir);
    }
    init_logging(&paths);

    let settings = settings::load(&paths.settings_file);
    tracing::info!(
        mode = ?paths.mode,
        data_dir = %paths.data_dir.display(),
        "starting PixTrace"
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new(paths, settings))
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_workspace_state,
            commands::open_workspace,
            commands::rescan_workspace,
            commands::rebuild_index,
            commands::get_cache_stats,
            commands::clear_cache,
            commands::set_cache_limit,
            commands::set_watch_enabled,
            commands::delete_images,
            commands::list_images,
            commands::search_images,
            commands::list_image_ids,
            commands::get_filter_options,
            commands::list_prompt_groups,
            commands::set_rating,
            commands::clear_rating,
            commands::get_image_detail,
            commands::get_image_asset,
            commands::get_index_progress,
            commands::cancel_indexing,
        ])
        .register_asynchronous_uri_scheme_protocol("pixtrace", |context, request, responder| {
            let app = context.app_handle().clone();
            std::thread::spawn(move || {
                responder.respond(protocol::handle(&app, &request));
            });
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_logging(paths: &Paths) {
    let file_appender = tracing_appender::rolling::daily(&paths.log_dir, "pixtrace.log");
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false);
    let stderr_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stderr_layer)
        .try_init();
}
