use keyring::Entry;

use crate::error::AppError;

const SERVICE: &str = "com.akshathsinha.relayrouting";

fn account_name(account_id: &str) -> String {
    format!("provider-account:{account_id}")
}

fn entry(account_id: &str) -> Result<Entry, AppError> {
    Entry::new(SERVICE, &account_name(account_id)).map_err(AppError::from)
}

pub fn set_secret(account_id: &str, secret: &str) -> Result<(), AppError> {
    entry(account_id)?.set_password(secret)?;
    Ok(())
}

pub fn get_secret(account_id: &str) -> Result<Option<String>, AppError> {
    match entry(account_id)?.get_password() {
        Ok(value) if !value.is_empty() => Ok(Some(value)),
        Ok(_) => Ok(None),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(AppError::from(err)),
    }
}

pub fn has_secret(account_id: &str) -> bool {
    matches!(get_secret(account_id), Ok(Some(_)))
}

pub fn delete_secret(account_id: &str) -> Result<(), AppError> {
    match entry(account_id)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(AppError::from(err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_names_are_scoped() {
        assert_eq!(account_name("abc"), "provider-account:abc");
        assert_eq!(SERVICE, "com.akshathsinha.relayrouting");
    }
}
