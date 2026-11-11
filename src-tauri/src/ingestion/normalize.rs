use crate::dedup::identity::canonical_identity;
use crate::error::AppError;
use crate::ingestion::adapter::{NormalizedJob, RawJob};
use crate::types::{Seniority, WorkModality};

const MAX_DESCRIPTION_CHARS: usize = 50_000;

/// Convert a source-native posting into the canonical normalized shape.
///
/// The raw description is consumed here and dropped; only cleaned text is kept.
pub fn normalize(
    raw: RawJob,
    source_id: &str,
    source_name: &str,
) -> Result<NormalizedJob, AppError> {
    let title = collapse_whitespace(&raw.title);
    if title.is_empty() {
        return Err(AppError::validation("job title cannot be empty"));
    }

    let company = collapse_whitespace(&raw.company);
    let company = if company.is_empty() {
        "Unknown".to_string()
    } else {
        company
    };

    let location = collapse_whitespace(&raw.location);
    let location = if location.is_empty() {
        "Unknown".to_string()
    } else {
        location
    };

    let url = raw.url.trim().to_string();
    if !is_http_url(&url) {
        return Err(AppError::validation(
            "job url must start with http:// or https://",
        ));
    }

    let description_clean = clean_description(&raw.description);
    let inferred_modality = infer_modality(
        &format!("{} {} {}", title, location, description_clean),
        raw.work_modality.as_deref(),
    );
    let seniority = detect_seniority(&format!("{title} {description_clean}"));

    let metadata = serde_json::json!({
        "seniority": seniority.map(Seniority::as_str),
        "work_modality": inferred_modality.as_str(),
        "location": location,
        "requirements": [],
    });

    let identity = canonical_identity(&company, &title, &location, &url);

    Ok(NormalizedJob {
        external_id: raw.external_id,
        title,
        company,
        location,
        work_modality: inferred_modality,
        url,
        posted_at: raw.posted_at.filter(|value| is_rfc3339(value)),
        description_clean,
        extracted_metadata_json: Some(metadata.to_string()),
        source_id: source_id.to_string(),
        source_name: source_name.to_string(),
        canonical_key: identity.key,
        canonical_hash: identity.hash,
        needs_review: identity.used_url_fallback,
    })
}

/// Strip markup, decode common entities, remove control characters, and
/// collapse whitespace. This is a controlled cleanup, not a full HTML parser.
pub fn clean_description(input: &str) -> String {
    let with_breaks = input
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n")
        .replace("</li>", "\n");

    let mut output = String::with_capacity(with_breaks.len());
    let mut inside_tag = false;
    for character in with_breaks.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if inside_tag => {}
            '\r' => {}
            '\n' | '\t' => output.push(character),
            character if character.is_control() => {}
            character => output.push(character),
        }
    }

    let decoded = output
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'");

    let mut result = String::with_capacity(decoded.len());
    let mut previous_was_space = false;
    let mut newline_count = 0;
    for character in decoded.chars() {
        if character == '\n' {
            newline_count += 1;
            if newline_count <= 2 {
                result.push('\n');
            }
            previous_was_space = true;
        } else if character.is_whitespace() {
            if !previous_was_space {
                result.push(' ');
                previous_was_space = true;
            }
            newline_count = 0;
        } else {
            result.push(character);
            previous_was_space = false;
            newline_count = 0;
        }
    }

    truncate_chars(result.trim(), MAX_DESCRIPTION_CHARS)
}

pub fn infer_modality(text: &str, explicit: Option<&str>) -> WorkModality {
    if let Some(value) = explicit {
        let parsed = WorkModality::parse(value);
        if parsed != WorkModality::Unknown {
            return parsed;
        }
    }

    let lower = text.to_ascii_lowercase();
    if lower.contains("hybrid") || lower.contains("híbrido") || lower.contains("hibrido") {
        WorkModality::Hybrid
    } else if lower.contains("remote")
        || lower.contains("remoto")
        || lower.contains("remota")
        || lower.contains("anywhere")
    {
        WorkModality::Remote
    } else if lower.contains("on-site") || lower.contains("onsite") || lower.contains("presencial")
    {
        WorkModality::Onsite
    } else {
        WorkModality::Unknown
    }
}

pub fn detect_seniority(text: &str) -> Option<Seniority> {
    let lower = text.to_ascii_lowercase();
    let normalized = lower.as_str();

    if contains_word(normalized, "trainee") {
        Some(Seniority::Trainee)
    } else if contains_word(normalized, "estágio")
        || contains_word(normalized, "estagio")
        || contains_word(normalized, "intern")
        || contains_word(normalized, "internship")
    {
        Some(Seniority::Intern)
    } else if contains_word(normalized, "pleno")
        || normalized.contains("mid-level")
        || normalized.contains("mid level")
    {
        Some(Seniority::Pleno)
    } else if contains_word(normalized, "júnior")
        || contains_word(normalized, "junior")
        || contains_word(normalized, "jr")
    {
        Some(Seniority::Junior)
    } else if contains_word(normalized, "senior")
        || contains_word(normalized, "sênior")
        || contains_word(normalized, "staff")
        || contains_word(normalized, "principal")
        || contains_word(normalized, "lead")
        || contains_word(normalized, "manager")
        || contains_word(normalized, "director")
        || normalized.contains("head of")
        || contains_word(normalized, "vp")
    {
        Some(Seniority::Senior)
    } else {
        None
    }
}

pub fn contains_word(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let mut start = 0;
    while let Some(relative) = haystack[start..].find(needle) {
        let index = start + relative;
        let before = haystack[..index].chars().next_back();
        let after = haystack[index + needle.len()..].chars().next();
        let before_boundary = before.is_none_or(|character| !character.is_alphanumeric());
        let after_boundary = after.is_none_or(|character| !character.is_alphanumeric());
        if before_boundary && after_boundary {
            return true;
        }
        start = index + needle.len();
    }
    false
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_http_url(value: &str) -> bool {
    (value.starts_with("http://") || value.starts_with("https://"))
        && value.len() > "https://".len()
        && !value.contains(char::is_whitespace)
}

fn is_rfc3339(value: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(value).is_ok()
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeroize::Zeroizing;

    fn raw(description: &str) -> RawJob {
        RawJob {
            external_id: "1".to_string(),
            title: "  Rust   Engineer ".to_string(),
            company: "Acme".to_string(),
            location: "Remote - Brazil".to_string(),
            work_modality: Some("Remote".to_string()),
            url: "https://example.com/1".to_string(),
            posted_at: Some("2026-11-08T12:00:00Z".to_string()),
            description: Zeroizing::new(description.to_string()),
            source_name: "test".to_string(),
            tags: vec![],
        }
    }

    #[test]
    fn cleans_markup_entities_and_control_characters() {
        let cleaned =
            clean_description("<p>Rust&nbsp;&amp; C++</p><script>x</script>\u{0007}<br>Remote");
        assert_eq!(cleaned, "Rust & C++\nx\nRemote");
        assert!(!cleaned.contains('<'));
    }

    #[test]
    fn infers_modality_and_seniority() {
        assert_eq!(infer_modality("Remoto Brasil", None), WorkModality::Remote);
        assert_eq!(infer_modality("modelo híbrido", None), WorkModality::Hybrid);
        assert_eq!(
            infer_modality("presencial", Some("Unknown")),
            WorkModality::Onsite
        );
        assert_eq!(
            detect_seniority("Desenvolvedor Pleno/Sênior"),
            Some(Seniority::Pleno)
        );
        assert_eq!(
            detect_seniority("Senior Rust Engineer"),
            Some(Seniority::Senior)
        );
        assert_eq!(detect_seniority("Estágio em TI"), Some(Seniority::Intern));
        assert_eq!(detect_seniority("Rust Engineer"), None);
    }

    #[test]
    fn rejects_invalid_urls() {
        let mut job = raw("Rust");
        job.url = "javascript:alert(1)".to_string();
        assert!(normalize(job, "s", "source").is_err());
    }

    #[test]
    fn normalized_job_carries_identity_and_no_raw_text() {
        let job = normalize(raw("<p>Rust role</p>"), "s", "source").expect("normalized");
        assert_eq!(job.title, "Rust Engineer");
        assert_eq!(job.description_clean, "Rust role");
        assert!(!job.canonical_hash.is_empty());
        assert!(job.extracted_metadata_json.is_some());
        assert!(!job.needs_review);
    }

    #[test]
    fn missing_identity_fields_flag_the_job_for_review() {
        let mut job = raw("Rust role");
        job.location = "Unknown".to_string();
        let normalized = normalize(job, "s", "source").expect("normalized");
        assert!(normalized.needs_review);
        assert!(normalized.canonical_key.starts_with("v1|url|"));
    }
}
