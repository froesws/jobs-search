//! Shared Tauri application state.
#![allow(missing_docs)]

use std::sync::{Mutex, RwLock};

use crate::config::{AppConfig, SessionSelection};
use crate::db::Db;
use crate::error::AppError;

/// Shared Tauri application state. The search lock enforces one run at a time;
/// database locks are only held for short synchronous queries.
pub struct AppState {
    pub db: Db,
    /// Effective configuration. Rebuilt when the operator saves settings so
    /// changes apply to the next run without restarting the app.
    pub config: RwLock<AppConfig>,
    /// Session-only provider/model override. Never persisted.
    pub session_selection: Mutex<Option<SessionSelection>>,
    pub search_lock: tokio::sync::Mutex<()>,
    pub http: reqwest::Client,
}

impl AppState {
    pub fn new(config: AppConfig) -> Result<Self, AppError> {
        let db = Db::open(&config.database_path)?;
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(
                config.llm_timeout_seconds.max(15),
            ))
            .user_agent("jobs-search/0.1")
            .build()
            .map_err(|_| AppError::Config("could not build the HTTP client".to_string()))?;

        Ok(Self {
            db,
            config: RwLock::new(config),
            session_selection: Mutex::new(None),
            search_lock: tokio::sync::Mutex::new(()),
            http,
        })
    }

    /// Snapshot the current configuration.
    pub fn config(&self) -> Result<AppConfig, AppError> {
        self.config
            .read()
            .map(|guard| guard.clone())
            .map_err(|_| AppError::Config("configuration lock poisoned".to_string()))
    }

    /// Replace the effective configuration.
    pub fn replace_config(&self, config: AppConfig) -> Result<(), AppError> {
        let mut guard = self
            .config
            .write()
            .map_err(|_| AppError::Config("configuration lock poisoned".to_string()))?;
        *guard = config;
        Ok(())
    }

    /// Current session-only selection, if any.
    pub fn session_selection(&self) -> Result<Option<SessionSelection>, AppError> {
        self.session_selection
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| AppError::Config("session selection lock poisoned".to_string()))
    }

    /// Replace the session-only selection.
    pub fn set_session_selection(
        &self,
        selection: Option<SessionSelection>,
    ) -> Result<(), AppError> {
        let mut guard = self
            .session_selection
            .lock()
            .map_err(|_| AppError::Config("session selection lock poisoned".to_string()))?;
        *guard = selection;
        Ok(())
    }
}
