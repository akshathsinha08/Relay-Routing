use std::collections::BTreeMap;
use std::sync::Arc;

use tauri::State;

use super::ctx;
use crate::database::models::UsageSummary;
use crate::error::AppError;
use crate::state::AppContext;

const KEYS: [&str; 8] = [
    "theme",
    "sendOnEnter",
    "contextMessageLimit",
    "maxOutputTokens",
    "appSystemPrompt",
    "onboardingComplete",
    "taskMaxSteps",
    "ollamaKeepAlive",
];

fn validate(key: &str, value: &str) -> Result<(), AppError> {
    if !KEYS.contains(&key) {
        return Err(AppError::invalid(format!("unknown setting: {key}")));
    }
    match key {
        "theme" => {
            if !matches!(value, "system" | "light" | "dark") {
                return Err(AppError::invalid("theme must be system, light or dark"));
            }
        }
        "sendOnEnter" | "onboardingComplete" => {
            if !matches!(value, "true" | "false") {
                return Err(AppError::invalid(format!("{key} must be true or false")));
            }
        }
        "contextMessageLimit" => {
            let parsed: i64 = value
                .parse()
                .map_err(|_| AppError::invalid("contextMessageLimit must be a number"))?;
            if !(2..=200).contains(&parsed) {
                return Err(AppError::invalid("contextMessageLimit must be between 2 and 200"));
            }
        }
        "maxOutputTokens" => {
            let parsed: i64 = value
                .parse()
                .map_err(|_| AppError::invalid("maxOutputTokens must be a number"))?;
            if !(64..=32768).contains(&parsed) {
                return Err(AppError::invalid("maxOutputTokens must be between 64 and 32768"));
            }
        }
        "taskMaxSteps" => {
            let parsed: i64 = value
                .parse()
                .map_err(|_| AppError::invalid("taskMaxSteps must be a number"))?;
            if !(1..=12).contains(&parsed) {
                return Err(AppError::invalid("taskMaxSteps must be between 1 and 12"));
            }
        }
        "appSystemPrompt" => {
            if value.len() > 20_000 {
                return Err(AppError::invalid("appSystemPrompt is too long"));
            }
        }
        "ollamaKeepAlive" if value.len() > 32 => {
            return Err(AppError::invalid("ollamaKeepAlive is too long"));
        }
        _ => {}
    }
    Ok(())
}

#[tauri::command]
pub fn get_settings(
    state: State<'_, Arc<AppContext>>,
) -> Result<BTreeMap<String, String>, AppError> {
    ctx(&state).db.get_settings()
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, Arc<AppContext>>,
    entries: BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, AppError> {
    let context = ctx(&state);
    for (key, value) in &entries {
        validate(key, value)?;
    }
    for (key, value) in &entries {
        context.db.set_setting(key, value)?;
    }
    context.db.get_settings()
}

#[tauri::command]
pub fn usage_summary(
    state: State<'_, Arc<AppContext>>,
) -> Result<Vec<UsageSummary>, AppError> {
    ctx(&state).db.usage_summary()
}
