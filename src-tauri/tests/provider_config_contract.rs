use jobs_search_lib::commands::config::build_view;
use jobs_search_lib::config::AppConfig;
use jobs_search_lib::state::AppState;

fn state(values: &[(&str, &str)]) -> (AppState, tempfile::TempDir) {
    let directory = tempfile::tempdir().expect("tempdir");
    let database = directory
        .path()
        .join("jobs-search.sqlite3")
        .display()
        .to_string();
    let mut map = values
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect::<std::collections::HashMap<_, _>>();
    map.insert("JOBS_SEARCH_DB_PATH".to_string(), database);
    let config = AppConfig::from_lookup(
        move |key| map.get(key).cloned(),
        directory.path().to_path_buf(),
        directory.path(),
    )
    .expect("config");
    let state = AppState::new(config).expect("state");
    (state, directory)
}

#[test]
fn provider_config_view_matches_the_contract_and_has_no_secret() {
    let (state, _dir) = state(&[
        ("GEMINI_API_KEY", "super-secret"),
        ("LLM_ALLOW_FALLBACK", "true"),
    ]);

    let view = build_view(&state).expect("view");
    let value = serde_json::to_value(&view).expect("serialize");

    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/contracts/provider-config.schema.json"
    ))
    .expect("schema");
    let validator = jsonschema::validator_for(&schema).expect("validator");
    validator.validate(&value).expect("contract");

    let serialized = value.to_string();
    assert!(!serialized.contains("super-secret"));
    assert_eq!(view.providers.len(), 4);
    assert!(view.fallback_enabled);
    assert!(view.session_override.is_none());
}

#[test]
fn provider_config_view_reports_a_session_override() {
    let (state, _dir) = state(&[("GEMINI_API_KEY", "secret")]);
    state
        .set_session_selection(Some(jobs_search_lib::config::SessionSelection {
            provider: "gemini".to_string(),
            models: vec!["gemini-custom".to_string()],
        }))
        .expect("set");

    let view = build_view(&state).expect("view");
    let value = serde_json::to_value(&view).expect("serialize");
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/contracts/provider-config.schema.json"
    ))
    .expect("schema");
    let validator = jsonschema::validator_for(&schema).expect("validator");
    validator.validate(&value).expect("contract");

    let override_view = view.session_override.expect("override");
    assert_eq!(override_view.provider, "gemini");
    assert_eq!(override_view.models, vec!["gemini-custom"]);
}
