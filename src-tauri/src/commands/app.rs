use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::database::models::{AppInfo, BackupInfo};
use crate::error::AppError;
use crate::state::AppContext;

#[tauri::command]
pub fn app_info(app: AppHandle, state: State<'_, Arc<AppContext>>) -> Result<AppInfo, AppError> {
    let ctx = state.inner();
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        identifier: app.config().identifier.clone(),
        platform: platform_name().to_string(),
        data_dir: ctx.paths.data_dir.to_string_lossy().into_owned(),
        db_path: ctx.paths.db_path.to_string_lossy().into_owned(),
        log_dir: ctx.paths.logs_dir.to_string_lossy().into_owned(),
        artifacts_dir: ctx.paths.artifacts_dir.to_string_lossy().into_owned(),
    })
}

fn platform_name() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        "linux"
    }
}

#[tauri::command]
pub fn health_check(state: State<'_, Arc<AppContext>>) -> Result<(), AppError> {
    state.inner().db.health_check()
}

#[tauri::command]
pub fn backup_database(state: State<'_, Arc<AppContext>>) -> Result<BackupInfo, AppError> {
    let ctx = state.inner();
    let dir = ctx.paths.data_dir.join("backups");
    std::fs::create_dir_all(&dir)?;
    ctx.db.backup_to(&dir)
}

#[tauri::command]
pub fn open_data_dir(state: State<'_, Arc<AppContext>>) -> Result<(), AppError> {
    tauri_plugin_opener::open_path(
        state.inner().paths.data_dir.clone(),
        None::<&str>,
    )
    .map_err(|error| AppError::internal(format!("cannot open data folder: {error}")))?;
    Ok(())
}
