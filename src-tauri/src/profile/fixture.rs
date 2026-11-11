use std::path::Path;

use crate::error::AppError;
use crate::profile::CandidateProfile;

/// Load the synthetic first-milestone profile.
///
/// A missing configuration or unreadable file blocks the search instead of
/// silently evaluating jobs against a fake built-in profile.
pub fn load(path: Option<&Path>) -> Result<CandidateProfile, AppError> {
    let Some(path) = path else {
        return Err(AppError::ProfileRequired);
    };

    let contents = std::fs::read_to_string(path).map_err(|error| {
        AppError::Config(format!(
            "could not read fixture profile {}: {error}",
            path.display()
        ))
    })?;
    load_from_str(&contents)
}

pub fn load_from_str(contents: &str) -> Result<CandidateProfile, AppError> {
    let profile: CandidateProfile = serde_json::from_str(contents)?;
    profile.validate()?;
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_path_blocks_the_search() {
        let error = load(None).expect_err("profile required");
        assert_eq!(error.code(), "PROFILE_REQUIRED");
    }

    #[test]
    fn invalid_profile_is_rejected() {
        let error = load_from_str("{}").expect_err("missing fields");
        assert_eq!(error.code(), "SERIALIZATION_ERROR");
    }
}
