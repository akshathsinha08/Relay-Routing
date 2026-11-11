mod commands;
mod database;
mod error;
mod providers;
mod security;
mod services;
mod state;
mod tasks;

use std::sync::Arc;

use tauri::Manager;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::database::Db;
use crate::state::{AppContext, AppPaths};

fn init_logging(paths: &AppPaths) {
    let appender = tracing_appender::rolling::daily(&paths.logs_dir, "relayrouting.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("relay_routing_lib=info,tauri=warn"));
    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_ansi(false),
        )
        .init();
    // The guard must outlive the process; leak it intentionally so logging is
    // never truncated during shutdown races.
    std::mem::forget(guard);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            let paths = AppPaths::resolve(app.handle())?;
            init_logging(&paths);
            let db = Db::open(&paths.db_path)?;
            let recovered = db.recover_interrupted_work()?;
            let interrupted = db.recover_interrupted_generations()?;
            if recovered > 0 || interrupted > 0 {
                tracing::info!(recovered, interrupted, "recovered interrupted work");
            }
            let context = AppContext::new(Arc::new(db), paths);
            app.manage(context);
            tracing::info!("Relay Routing started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_info,
            commands::app::health_check,
            commands::app::backup_database,
            commands::app::open_data_dir,
            commands::providers::list_providers,
            commands::providers::update_provider,
            commands::providers::list_accounts,
            commands::providers::create_account,
            commands::providers::update_account_secret,
            commands::providers::rename_account,
            commands::providers::delete_account,
            commands::providers::provider_health,
            commands::providers::test_account,
            commands::providers::list_models,
            commands::projects::list_projects,
            commands::projects::get_project,
            commands::projects::create_project,
            commands::projects::update_project,
            commands::projects::delete_project,
            commands::projects::list_project_attachments,
            commands::projects::list_project_artifacts,
            commands::conversations::list_conversations,
            commands::conversations::get_conversation,
            commands::conversations::create_conversation,
            commands::conversations::rename_conversation,
            commands::conversations::set_conversation_flags,
            commands::conversations::set_conversation_target,
            commands::conversations::move_conversation,
            commands::conversations::delete_conversation,
            commands::conversations::list_messages,
            commands::conversations::search_messages,
            commands::conversations::export_conversation,
            commands::conversations::send_message,
            commands::conversations::continue_generation,
            commands::conversations::cancel_stream,
            commands::attachments::import_attachments,
            commands::attachments::delete_attachment,
            commands::attachments::attachment_preview,
            commands::attachments::open_attachment,
            commands::tasks::list_tasks,
            commands::tasks::get_task_detail,
            commands::tasks::create_task,
            commands::tasks::start_task,
            commands::tasks::pause_task,
            commands::tasks::cancel_task,
            commands::tasks::resume_task,
            commands::tasks::delete_task,
            commands::tasks::task_artifacts,
            commands::tasks::open_artifact,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::usage_summary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
