use std::sync::Arc;
use std::time::Instant;

use tauri::State;

use super::ctx;
use crate::database::models::{Account, AccountTest, Provider, ProviderHealth};
use crate::error::AppError;
use crate::providers::{self, ModelInfo};
use crate::security::keychain;
use crate::services::accounts::{config_for_account, decorate_account, provider_error, resolve_account};
use crate::state::AppContext;

#[tauri::command]
pub fn list_providers(state: State<'_, Arc<AppContext>>) -> Result<Vec<Provider>, AppError> {
    ctx(&state).db.list_providers()
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_provider(
    state: State<'_, Arc<AppContext>>,
    id: String,
    name: Option<String>,
    base_url: Option<String>,
    enabled: Option<bool>,
) -> Result<Provider, AppError> {
    ctx(&state)
        .db
        .update_provider(&id, name.as_deref(), base_url.as_deref(), enabled)
}

#[tauri::command]
pub fn list_accounts(
    state: State<'_, Arc<AppContext>>,
    provider_id: Option<String>,
) -> Result<Vec<Account>, AppError> {
    let accounts = ctx(&state).db.list_accounts(provider_id.as_deref())?;
    Ok(accounts.into_iter().map(decorate_account).collect())
}

#[tauri::command]
pub fn create_account(
    state: State<'_, Arc<AppContext>>,
    provider_id: String,
    label: String,
    auth_kind: String,
    secret: Option<String>,
) -> Result<Account, AppError> {
    let context = ctx(&state);
    let account = context
        .db
        .create_account(&provider_id, &label, &auth_kind, "unconfigured")?;
    if let Some(secret) = secret.map(|value| value.trim().to_string()).filter(|v| !v.is_empty()) {
        if account.auth_kind != "api_key" {
            return Err(AppError::invalid("this provider does not use API keys"));
        }
        keychain::set_secret(&account.id, &secret)?;
    }
    Ok(decorate_account(account))
}

#[tauri::command]
pub fn update_account_secret(
    state: State<'_, Arc<AppContext>>,
    account_id: String,
    secret: String,
) -> Result<Account, AppError> {
    let context = ctx(&state);
    let account = context.db.get_account(&account_id)?;
    if account.auth_kind != "api_key" {
        return Err(AppError::invalid("this provider does not use API keys"));
    }
    let secret = secret.trim();
    if secret.is_empty() {
        keychain::delete_secret(&account_id)?;
        context
            .db
            .update_account(&account_id, None, Some("unconfigured"), Some(""))?;
        return Ok(decorate_account(context.db.get_account(&account_id)?));
    }
    keychain::set_secret(&account_id, secret)?;
    let account = context
        .db
        .update_account(&account_id, None, None, Some(""))?;
    Ok(decorate_account(account))
}

#[tauri::command]
pub fn rename_account(
    state: State<'_, Arc<AppContext>>,
    account_id: String,
    label: String,
) -> Result<Account, AppError> {
    let context = ctx(&state);
    let account = context.db.update_account(&account_id, Some(&label), None, None)?;
    Ok(decorate_account(account))
}

#[tauri::command]
pub fn delete_account(
    state: State<'_, Arc<AppContext>>,
    account_id: String,
) -> Result<(), AppError> {
    let context = ctx(&state);
    context.db.get_account(&account_id)?;
    keychain::delete_secret(&account_id)?;
    context.db.delete_account(&account_id)
}

#[tauri::command]
pub async fn provider_health(
    state: State<'_, Arc<AppContext>>,
    provider_id: String,
    account_id: Option<String>,
) -> Result<ProviderHealth, AppError> {
    let context = ctx(&state);
    let provider = context.db.get_provider(&provider_id)?;
    let keyless = kind_is_keyless(&provider.kind);
    let (kind, config) = match resolve_account(&context, &provider_id, account_id.as_deref()) {
        Ok((_, account)) => config_for_account(&context, &account)?,
        Err(error) => {
            if !keyless {
                return Err(error);
            }
            (
                crate::providers::Kind::parse(&provider.kind).map_err(provider_error)?,
                crate::providers::ProviderConfig {
                    base_url: provider.base_url.clone(),
                    api_key: None,
                },
            )
        }
    };
    match providers::health_for(kind, &context.http, &config).await {
        Ok((version, latency_ms)) => Ok(ProviderHealth {
            ok: true,
            message: format!("Connected to {version}"),
            latency_ms,
            version: Some(version),
        }),
        Err(error) => Ok(ProviderHealth {
            ok: false,
            message: error.to_string(),
            latency_ms: 0,
            version: None,
        }),
    }
}

fn kind_is_keyless(kind: &str) -> bool {
    kind == "ollama"
}

#[tauri::command]
pub async fn test_account(
    state: State<'_, Arc<AppContext>>,
    account_id: String,
) -> Result<AccountTest, AppError> {
    let context = ctx(&state);
    let account = context.db.get_account(&account_id)?;
    let (kind, config) = config_for_account(&context, &account)?;
    let started = Instant::now();
    let result = providers::health_for(kind, &context.http, &config).await;
    let latency_ms = started.elapsed().as_millis() as u64;
    match result {
        Ok((version, _)) => {
            let model_count = providers::list_models_for(kind, &context.http, &config)
                .await
                .map(|models| models.len())
                .unwrap_or(0);
            let _ = context.db.update_account(
                &account_id,
                None,
                Some("ok"),
                Some(&format!("connected (version {version})")),
            );
            Ok(AccountTest {
                ok: true,
                message: format!("Connected. Found {model_count} model(s)."),
                latency_ms,
                model_count,
            })
        }
        Err(error) => {
            let message = error.to_string();
            let _ = context
                .db
                .update_account(&account_id, None, Some("error"), Some(&message));
            Ok(AccountTest {
                ok: false,
                message,
                latency_ms,
                model_count: 0,
            })
        }
    }
}

#[tauri::command]
pub async fn list_models(
    state: State<'_, Arc<AppContext>>,
    provider_id: String,
    account_id: Option<String>,
) -> Result<Vec<ModelInfo>, AppError> {
    let context = ctx(&state);
    let (_, account) = resolve_account(&context, &provider_id, account_id.as_deref())?;
    let (kind, config) = config_for_account(&context, &account)?;
    providers::list_models_for(kind, &context.http, &config)
        .await
        .map_err(provider_error)
}
