pub const MAX_MESSAGE_LEN: usize = 512;
pub const MAX_IDENTIFIER_LEN: usize = 128;

/// Return a safe message, or `[redacted]` when the input contains content that
/// must never reach the log file.
pub fn sanitize_message(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if contains_forbidden(trimmed) {
        return Some("[redacted]".to_string());
    }
    Some(truncate(trimmed, MAX_MESSAGE_LEN))
}

/// Return a safe opaque identifier, or `None` when the value contains
/// whitespace, a URL, a path, or another forbidden pattern.
pub fn sanitize_identifier(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > MAX_IDENTIFIER_LEN
        || trimmed.chars().any(char::is_whitespace)
        || contains_forbidden(trimmed)
    {
        return None;
    }
    Some(trimmed.to_string())
}

pub fn contains_forbidden(input: &str) -> bool {
    let lower = input.to_ascii_lowercase();
    const FORBIDDEN: &[&str] = &[
        "http://",
        "https://",
        "file://",
        "/home/",
        "/users/",
        "c:\\",
        "api_key",
        "api-key",
        "authorization",
        "password",
        "secret",
        "token=",
        "prompt",
        "response",
        "description",
        "candidate",
        "@",
    ];
    FORBIDDEN.iter().any(|pattern| lower.contains(pattern))
}

fn truncate(input: &str, max: usize) -> String {
    input.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_urls_paths_prompts_and_candidate_data() {
        for input in [
            "see https://example.com/job",
            "file /home/will/resume.pdf",
            "prompt: tell me everything",
            "candidate email a@b.com",
            "api_key=abc",
        ] {
            assert_eq!(sanitize_message(input).as_deref(), Some("[redacted]"));
        }
    }

    #[test]
    fn keeps_safe_technical_messages() {
        assert_eq!(
            sanitize_message("source request failed").as_deref(),
            Some("source request failed")
        );
    }

    #[test]
    fn truncates_at_512_characters() {
        let input = "a".repeat(600);
        let output = sanitize_message(&input).unwrap();
        assert_eq!(output.chars().count(), 512);
    }

    #[test]
    fn identifiers_reject_urls_whitespace_and_paths() {
        assert!(sanitize_identifier("run-1").is_some());
        assert!(sanitize_identifier("run 1").is_none());
        assert!(sanitize_identifier("https://example.com").is_none());
        assert!(sanitize_identifier("/home/will").is_none());
    }
}
