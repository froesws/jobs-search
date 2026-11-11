use tauri::State;

use crate::error::ErrorPayload;
use crate::profile::CandidateProfile;
use crate::profile::fixture;
use crate::state::AppState;

#[tauri::command]
pub fn cmd_get_profile(state: State<'_, AppState>) -> Result<CandidateProfile, ErrorPayload> {
    fixture::load(state.config.fixture_profile_path.as_deref()).map_err(Into::into)
}
