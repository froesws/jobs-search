use tauri::Manager;

pub mod commands;
pub mod config;
pub mod db;
pub mod dedup;
pub mod error;
pub mod ingestion;
pub mod llm;
#[path = "match/mod.rs"]
pub mod matching;
pub mod pdf;
pub mod profile;
pub mod state;
pub mod telemetry;
pub mod types;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Jobs Search");
}
