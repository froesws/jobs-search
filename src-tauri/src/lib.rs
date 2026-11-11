//! Jobs Search desktop application: multi-source job discovery, LLM-powered
//! evaluation, and application tracking.

#![warn(missing_docs)]

use tauri::Manager;

/// Dashboards, funnels, and CSV export.
pub mod analytics;
pub mod commands;
pub mod config;
pub mod credentials;
pub mod db;
pub mod dedup;
/// Application error type and wire payload.
pub mod error;
pub mod ingestion;
pub mod llm;
#[path = "match/mod.rs"]
pub mod matching;
pub mod pdf;
pub mod profile;
pub mod state;
pub mod telemetry;
/// Core domain types shared across the command boundary.
pub mod types;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Bootstrap and run the Tauri application.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let log_dir = app_data_dir.join("logs");
            let (config, warning) = telemetry::TelemetryConfig::from_env(log_dir);
            let runtime = match telemetry::init(config) {
                Ok(runtime) => runtime,
                Err(error) => telemetry::TelemetryRuntime::degraded(error.message()),
            };

            if let Some(warning) = warning {
                telemetry::Telemetry::config_warning(&warning);
            }

            app.manage(runtime.health.clone());
            app.manage(runtime.guard);

            let config = config::AppConfig::from_env(app_data_dir)?;
            let state = state::AppState::new(config)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::telemetry::cmd_get_log_health,
            commands::profile::cmd_get_profile,
            commands::profile::cmd_list_resumes,
            commands::profile::cmd_upload_resume,
            commands::profile::cmd_delete_resume,
            commands::profile::cmd_save_profile,
            commands::search::cmd_trigger_search,
            commands::jobs::cmd_get_jobs,
            commands::jobs::cmd_get_job,
            commands::jobs::cmd_apply_job,
            commands::jobs::cmd_ignore_job,
            commands::jobs::cmd_restore_job,
            commands::jobs::cmd_approve_job,
            commands::jobs::cmd_get_audit,
            commands::config::cmd_get_active_provider,
            commands::config::cmd_get_provider_config,
            commands::config::cmd_save_provider_config,
            commands::config::cmd_save_provider_credential,
            commands::config::cmd_remove_provider_credential,
            commands::config::cmd_reset_provider_setting,
            commands::config::cmd_refresh_provider_models,
            commands::config::cmd_test_provider,
            commands::config::cmd_set_session_selection,
            commands::analytics::cmd_get_analytics,
            commands::analytics::cmd_export_analytics_csv,
        ])
        // Panicking here is intentional: a missing Tauri context is a build or
        // packaging invariant, not a recoverable runtime condition.
        .run(tauri::generate_context!())
        .expect("error while running Jobs Search");
}
