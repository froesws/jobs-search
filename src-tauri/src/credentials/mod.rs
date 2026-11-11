//! Provider credential storage.
//!
//! Credentials live in the OS credential store (Secret Service on Linux). The
//! local `.env` file is a read-only bootstrap source and a consented plaintext
//! fallback: the app only writes or removes entries after an explicit operator
//! confirmation.

use std::path::{Path, PathBuf};

use crate::error::AppError;

/// Service name used for every credential-store entry.
pub const SERVICE: &str = "jobs-search";

/// Where a provider credential was resolved from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
    /// The OS credential store.
    CredentialStore,
    /// The local `.env` file.
    Env,
    /// No credential is configured.
    None,
}

impl CredentialSource {
    /// Stable wire label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CredentialStore => "credential_store",
            Self::Env => "env",
            Self::None => "none",
        }
    }
}

/// Credential-store account name for a provider.
pub fn account(provider: &str) -> String {
    format!("llm:{provider}")
}

/// Environment variable name for a provider key.
pub fn env_key(provider: &str) -> Option<&'static str> {
    match provider {
        "gemini" => Some("GEMINI_API_KEY"),
        "openrouter" => Some("OPENROUTER_API_KEY"),
        "deepseek" => Some("DEEPSEEK_API_KEY"),
        _ => None,
    }
}

/// Read a credential from the OS store.
///
/// Returns `Ok(None)` when the entry is absent and
/// `Err(AppError::CredentialStoreUnavailable)` when the store cannot be reached.
pub fn get_from_store(provider: &str) -> Result<Option<String>, AppError> {
    let entry = keyring::Entry::new(SERVICE, &account(provider))
        .map_err(|_| AppError::CredentialStoreUnavailable)?;
    match entry.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err(AppError::CredentialStoreUnavailable),
    }
}

/// Store a credential in the OS store.
pub fn set_in_store(provider: &str, secret: &str) -> Result<(), AppError> {
    let entry = keyring::Entry::new(SERVICE, &account(provider))
        .map_err(|_| AppError::CredentialStoreUnavailable)?;
    entry
        .set_password(secret)
        .map_err(|_| AppError::CredentialStoreUnavailable)
}

/// Delete a credential from the OS store. A missing entry is not an error.
pub fn delete_from_store(provider: &str) -> Result<(), AppError> {
    let entry = keyring::Entry::new(SERVICE, &account(provider))
        .map_err(|_| AppError::CredentialStoreUnavailable)?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(AppError::CredentialStoreUnavailable),
    }
}

/// Resolve a credential, preferring the OS store and falling back to `.env`.
pub fn resolve(provider: &str, env_file: Option<&Path>) -> (Option<String>, CredentialSource) {
    if let Ok(Some(secret)) = get_from_store(provider) {
        return (Some(secret), CredentialSource::CredentialStore);
    }
    if let Some(key) = env_key(provider)
        && let Some(path) = env_file
        && let Some(value) = read_env_value(path, key)
    {
        return (Some(value), CredentialSource::Env);
    }
    (None, CredentialSource::None)
}

/// Find the nearest `.env` in the working directory or an ancestor.
pub fn discover_env_file(working_dir: &Path) -> Option<PathBuf> {
    working_dir
        .ancestors()
        .map(|directory| directory.join(".env"))
        .find(|candidate| candidate.is_file())
}

/// Read a single value from a `.env` file.
pub fn read_env_value(path: &Path, key: &str) -> Option<String> {
    let contents = std::fs::read_to_string(path).ok()?;
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        let Some((name, value)) = trimmed.split_once('=') else {
            continue;
        };
        if name.trim() == key {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if value.is_empty() {
                return None;
            }
            return Some(value.to_string());
        }
    }
    None
}

/// Write or replace a value in a `.env` file, keeping other lines intact.
///
/// The file is created with `0600` permissions when missing and re-secured when
/// it already exists.
pub fn write_env_value(path: &Path, key: &str, value: &str) -> Result<(), AppError> {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<String> = existing.lines().map(str::to_string).collect();
    let mut replaced = false;
    for line in &mut lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = trimmed.split_once('=')
            && name.trim() == key
        {
            *line = format!("{key}={value}");
            replaced = true;
            break;
        }
    }
    if !replaced {
        lines.push(format!("{key}={value}"));
    }
    let mut contents = lines.join("\n");
    contents.push('\n');
    write_private(path, &contents)
}

/// Remove a key from a `.env` file. A missing key or file is not an error.
pub fn remove_env_value(path: &Path, key: &str) -> Result<(), AppError> {
    let Ok(existing) = std::fs::read_to_string(path) else {
        return Ok(());
    };
    let lines: Vec<&str> = existing
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                return true;
            }
            match trimmed.split_once('=') {
                Some((name, _)) => name.trim() != key,
                None => true,
            }
        })
        .collect();
    let mut contents = lines.join("\n");
    if !contents.is_empty() {
        contents.push('\n');
    }
    write_private(path, &contents)
}

fn write_private(path: &Path, contents: &str) -> Result<(), AppError> {
    std::fs::write(path, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_and_env_key_are_stable() {
        assert_eq!(account("gemini"), "llm:gemini");
        assert_eq!(env_key("deepseek"), Some("DEEPSEEK_API_KEY"));
        assert_eq!(env_key("ollama"), None);
    }

    #[test]
    fn env_values_are_read_written_and_removed() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join(".env");
        std::fs::write(&path, "# comment\nGEMINI_API_KEY=old\nOTHER=1\n").expect("seed");

        assert_eq!(
            read_env_value(&path, "GEMINI_API_KEY"),
            Some("old".to_string())
        );

        write_env_value(&path, "GEMINI_API_KEY", "new").expect("write");
        let contents = std::fs::read_to_string(&path).expect("read");
        assert!(contents.contains("GEMINI_API_KEY=new"));
        assert!(contents.contains("# comment"));
        assert!(contents.contains("OTHER=1"));
        assert_eq!(contents.matches("GEMINI_API_KEY").count(), 1);

        write_env_value(&path, "DEEPSEEK_API_KEY", "added").expect("append");
        assert_eq!(
            read_env_value(&path, "DEEPSEEK_API_KEY"),
            Some("added".to_string())
        );

        remove_env_value(&path, "GEMINI_API_KEY").expect("remove");
        assert_eq!(read_env_value(&path, "GEMINI_API_KEY"), None);
        assert!(
            std::fs::read_to_string(&path)
                .expect("read")
                .contains("OTHER=1")
        );
    }

    #[test]
    fn env_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join(".env");
        write_env_value(&path, "GEMINI_API_KEY", "secret").expect("write");
        let mode = std::fs::metadata(&path).expect("meta").permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn discover_env_file_walks_up() {
        let directory = tempfile::tempdir().expect("tempdir");
        let nested = directory.path().join("a/b");
        std::fs::create_dir_all(&nested).expect("dirs");
        std::fs::write(directory.path().join(".env"), "X=1\n").expect("seed");
        assert_eq!(
            discover_env_file(&nested),
            Some(directory.path().join(".env"))
        );
    }

    #[test]
    fn empty_values_are_treated_as_missing() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join(".env");
        std::fs::write(&path, "GEMINI_API_KEY=\n").expect("seed");
        assert_eq!(read_env_value(&path, "GEMINI_API_KEY"), None);
    }
}
