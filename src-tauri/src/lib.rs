use tauri::Manager;

pub mod commands;
pub mod error;
pub mod telemetry;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let log_dir = app.path().app_data_dir()?.join("logs");
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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::telemetry::cmd_get_log_health
        ])
        .run(tauri::generate_context!())
        .expect("error while running Jobs Search");
}
