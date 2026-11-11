//! Canonical identity derivation and hashing.
#![allow(missing_docs)]

use sha2::{Digest, Sha256};

/// The stable identity of a canonical job posting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalIdentity {
    pub key: String,
    pub hash: String,
    pub used_url_fallback: bool,
}

/// Company + title + location, with a normalized-URL fallback when location or
/// a required identity field is missing.
pub fn canonical_identity(
    company: &str,
    title: &str,
    location: &str,
    url: &str,
) -> CanonicalIdentity {
    let company_key = normalize_text(company);
    let title_key = normalize_text(title);
    let location_key = normalize_text(location);
    let location_missing = location_key.is_empty() || location_key == "unknown";
    let use_url = company_key.is_empty() || title_key.is_empty() || location_missing;

    let key = if use_url {
        format!("v1|url|{}", normalize_url(url))
    } else {
        format!("v1|{company_key}|{title_key}|{location_key}")
    };

    CanonicalIdentity {
        hash: sha256_hex(&key),
        key,
        used_url_fallback: use_url,
    }
}

/// Normalize display text for identity comparison: case-folded, diacritics
/// removed, punctuation collapsed to single spaces.
pub fn normalize_text(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        let replacement = match character {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            other => other,
        };
        if replacement.is_alphanumeric() {
            for lower in replacement.to_lowercase() {
                output.push(lower);
            }
        } else if !output.ends_with(' ') {
            output.push(' ');
        }
    }
    output.trim().to_string()
}

/// Normalize a posting URL for fallback identity: lowercases scheme/host,
/// removes fragments and common tracking parameters, and trims a trailing slash.
pub fn normalize_url(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let without_fragment = trimmed.split('#').next().unwrap_or(trimmed);
    let (base, query) = match without_fragment.split_once('?') {
        Some((base, query)) => (base, Some(query)),
        None => (without_fragment, None),
    };

    let normalized_base = match base.split_once("://") {
        Some((scheme, rest)) => {
            let scheme = scheme.to_ascii_lowercase();
            let (host, path) = match rest.split_once('/') {
                Some((host, path)) => (host.to_ascii_lowercase(), format!("/{path}")),
                None => (rest.to_ascii_lowercase(), String::new()),
            };
            format!("{scheme}://{host}{path}")
        }
        None => base.to_ascii_lowercase(),
    };

    let kept_query: Vec<&str> = query
        .map(|query| {
            query
                .split('&')
                .filter(|pair| !is_tracking_parameter(pair))
                .collect()
        })
        .unwrap_or_default();

    let mut result = normalized_base.trim_end_matches('/').to_string();
    if !kept_query.is_empty() {
        result.push('?');
        result.push_str(&kept_query.join("&"));
    }
    result
}

fn is_tracking_parameter(pair: &str) -> bool {
    let key = pair
        .split_once('=')
        .map(|(key, _)| key)
        .unwrap_or(pair)
        .to_ascii_lowercase();
    key.starts_with("utm_")
        || matches!(
            key.as_str(),
            "ref" | "refid" | "referrer" | "source" | "trk" | "trackingid" | "gclid" | "fbclid"
        )
}

pub fn sha256_hex(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    hex_digest(digest)
}

pub fn sha256_bytes(input: &[u8]) -> String {
    let digest = Sha256::digest(input);
    hex_digest(digest)
}

fn hex_digest(digest: impl AsRef<[u8]>) -> String {
    let mut output = String::with_capacity(digest.as_ref().len() * 2);
    for byte in digest.as_ref() {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_source_identity_ignores_case_punctuation_and_accents() {
        let first = canonical_identity(
            "Acme, Inc.",
            "Desenvolvedor Sênior",
            "São Paulo",
            "https://a.example/1",
        );
        let second = canonical_identity(
            "acme inc",
            "desenvolvedor senior",
            "sao paulo",
            "https://b.example/2",
        );
        assert_eq!(first.hash, second.hash);
        assert!(!first.used_url_fallback);
    }

    #[test]
    fn missing_location_uses_url_fallback() {
        let identity = canonical_identity("Acme", "Rust Engineer", "Unknown", "https://a/1");
        assert!(identity.used_url_fallback);
        assert!(identity.key.contains("https://a/1"));
    }

    #[test]
    fn tracking_parameters_do_not_create_duplicates() {
        let first = canonical_identity(
            "Acme",
            "Rust Engineer",
            "Remote",
            "https://a/1?utm_source=x",
        );
        let second = canonical_identity("Acme", "Rust Engineer", "Remote", "https://a/1");
        assert_eq!(first.hash, second.hash);
    }
}
