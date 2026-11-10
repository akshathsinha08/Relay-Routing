use crate::database::models::{Account, Conversation, Provider};
use crate::error::AppError;
use crate::providers::{Kind, ProviderConfig, ProviderError};
use crate::security::keychain;
use crate::state::AppContext;

#[derive(Debug, Clone)]
pub struct Target {
    pub provider_id: String,
    pub account_id: String,
    pub model: String,
    pub kind: Kind,
    pub config: ProviderConfig,
}

pub fn provider_error(error: ProviderError) -> AppError {
    AppError::new(error.code(), error.to_string())
}

pub fn config_for_account(
    ctx: &AppContext,
    account: &Account,
) -> Result<(Kind, ProviderConfig), AppError> {
    let provider = ctx.db.get_provider(&account.provider_id)?;
    if !provider.enabled {
        return Err(AppError::invalid("this provider is disabled"));
    }
    let kind = Kind::parse(&provider.kind).map_err(provider_error)?;
    let api_key = if account.auth_kind == "api_key" {
        Some(keychain::get_secret(&account.id)?.ok_or_else(|| {
            AppError::new("missing_api_key", "add an API key for this account")
        })?)
    } else {
        None
    };
    Ok((
        kind,
        ProviderConfig {
            base_url: provider.base_url,
            api_key,
        },
    ))
}

pub fn decorate_account(mut account: Account) -> Account {
    account.has_secret = keychain::has_secret(&account.id);
    account
}

pub fn resolve_account(
    ctx: &AppContext,
    provider_id: &str,
    account_id: Option<&str>,
) -> Result<(Provider, Account), AppError> {
    let provider = ctx.db.get_provider(provider_id)?;
    let account = match account_id {
        Some(id) => {
            let account = ctx.db.get_account(id)?;
            if account.provider_id != provider_id {
                return Err(AppError::invalid(
                    "account does not belong to the selected provider",
                ));
            }
            account
        }
        None => ctx
            .db
            .list_accounts(Some(provider_id))?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::invalid("add an account for this provider first"))?,
    };
    Ok((provider, account))
}

/// Resolve one explicit or first eligible account for a task's fixed provider/model.
pub fn target_for(
    ctx: &AppContext,
    provider_id: &str,
    account_id: Option<&str>,
    model: &str,
) -> Result<Target, AppError> {
    let model = model.trim();
    if model.is_empty() {
        return Err(AppError::invalid("choose a model for this task"));
    }
    let provider = ctx.db.get_provider(provider_id)?;
    if !provider.enabled {
        return Err(AppError::invalid("this provider is disabled"));
    }
    let kind = Kind::parse(&provider.kind).map_err(provider_error)?;
    let mut accounts = ctx.db.list_accounts(Some(provider_id))?;
    if let Some(preferred) = account_id {
        accounts.sort_by_key(|account| i64::from(account.id != preferred));
    }
    for account in accounts {
        if account.status == "disabled" {
            continue;
        }
        if account.auth_kind == "api_key" {
            match keychain::get_secret(&account.id) {
                Ok(Some(api_key)) => {
                    return Ok(Target {
                        provider_id: provider.id.clone(),
                        account_id: account.id,
                        model: model.to_string(),
                        kind,
                        config: ProviderConfig {
                            base_url: provider.base_url,
                            api_key: Some(api_key),
                        },
                    });
                }
                Ok(None) | Err(_) => continue,
            }
        }
        return Ok(Target {
            provider_id: provider.id.clone(),
            account_id: account.id,
            model: model.to_string(),
            kind,
            config: ProviderConfig {
                base_url: provider.base_url,
                api_key: None,
            },
        });
    }
    Err(AppError::invalid("no eligible account for this provider"))
}

/// Resolve every eligible account for a conversation's fixed provider/model.
///
/// The conversation's account is a preference, not a binding: if it is missing,
/// disabled, or out of credentials, other accounts on the same provider are used.
pub fn targets_for(
    ctx: &AppContext,
    conversation: &Conversation,
) -> Result<Vec<Target>, AppError> {
    let provider_id = conversation
        .provider_id
        .as_deref()
        .ok_or_else(|| AppError::invalid("choose a provider for this conversation"))?;
    let model = conversation
        .model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::invalid("choose a model for this conversation"))?
        .to_string();
    let provider = ctx.db.get_provider(provider_id)?;
    if !provider.enabled {
        return Err(AppError::invalid("this provider is disabled"));
    }
    let kind = Kind::parse(&provider.kind).map_err(|error| AppError::new(error.code(), error.to_string()))?;

    let mut accounts = ctx.db.list_accounts(Some(&provider.id))?;
    if let Some(preferred) = conversation.account_id.as_deref() {
        accounts.sort_by_key(|account| i64::from(account.id != preferred));
    }

    let mut targets = Vec::new();
    let mut saw_configured_account = false;
    let mut saw_missing_secret = false;
    for account in accounts {
        if account.status == "disabled" {
            continue;
        }
        saw_configured_account = true;
        let api_key = if account.auth_kind == "api_key" {
            match keychain::get_secret(&account.id) {
                Ok(Some(secret)) => Some(secret),
                Ok(None) => {
                    saw_missing_secret = true;
                    continue;
                }
                Err(_) => {
                    saw_missing_secret = true;
                    continue;
                }
            }
        } else {
            None
        };
        if targets
            .iter()
            .any(|target: &Target| target.account_id == account.id)
        {
            continue;
        }
        targets.push(Target {
            provider_id: provider.id.clone(),
            account_id: account.id,
            model: model.clone(),
            kind,
            config: ProviderConfig {
                base_url: provider.base_url.clone(),
                api_key,
            },
        });
    }

    if targets.is_empty() {
        if saw_missing_secret {
            return Err(AppError::new(
                "missing_api_key",
                "add an API key for this provider's account",
            ));
        }
        if saw_configured_account {
            return Err(AppError::invalid("no eligible account for this provider"));
        }
        return Err(AppError::invalid("add an account for this provider first"));
    }
    Ok(targets)
}

/// Only transient transport or server failures are retried automatically.
/// Usage limits and authentication failures require an explicit user action.
pub fn is_retryable(error: &ProviderError) -> bool {
    match error {
        ProviderError::Network(_) => true,
        ProviderError::Http { status, .. } => (500..600).contains(status),
        ProviderError::BadResponse(message) => {
            let lower = message.to_ascii_lowercase();
            !lower.contains("unsupported provider kind")
        }
        ProviderError::Auth(_)
        | ProviderError::RateLimited(_)
        | ProviderError::NotConfigured(_)
        | ProviderError::Cancelled => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_only_transient_failures() {
        assert!(is_retryable(&ProviderError::Network("reset".into())));
        assert!(is_retryable(&ProviderError::Http {
            status: 503,
            message: "unavailable".into()
        }));
        assert!(!is_retryable(&ProviderError::Http {
            status: 400,
            message: "bad request".into()
        }));
        assert!(!is_retryable(&ProviderError::RateLimited("slow down".into())));
        assert!(!is_retryable(&ProviderError::Auth("denied".into())));
        assert!(!is_retryable(&ProviderError::Cancelled));
    }
}
