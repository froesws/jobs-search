use crate::config::AppConfig;
use crate::db::Db;
use crate::error::AppError;

/// Shared Tauri application state. The search lock enforces one run at a time;
/// database locks are only held for short synchronous queries.
pub struct AppState {
    pub db: Db,
    pub config: AppConfig,
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
            config,
            search_lock: tokio::sync::Mutex::new(()),
            http,
        })
    }
}
