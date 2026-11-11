#![allow(dead_code)]

use std::path::PathBuf;

use serde_json::Value;

pub fn contract_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/contracts")
        .join(name)
}

pub fn load_contract(name: &str) -> Value {
    let path = contract_path(name);
    let contents = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()));
    serde_json::from_str(&contents).expect("contract is valid JSON")
}

pub fn assert_valid(instance: &Value, contract: &str) {
    let schema = load_contract(contract);
    let validator = jsonschema::validator_for(&schema).expect("valid JSON schema");
    validator.validate(instance).unwrap_or_else(|error| {
        panic!("instance does not match {contract}: {error}");
    });
}

#[test]
fn evaluation_contract_accepts_the_strict_payload() {
    let payload = serde_json::json!({
        "seniority": "Pleno",
        "work_modality": "Remote",
        "location": "Remote - Brazil",
        "required_skills": ["Rust"],
        "desirable_skills": ["Docker"],
        "requirements": [
            {
                "id": "r1",
                "text": "Rust",
                "dimension": "CORE_TECHNICAL",
                "match_type": "EXACT",
                "evidence": "five years"
            }
        ],
        "summary": "Strong fit"
    });
    assert_valid(&payload, "llm-evaluation.schema.json");
}
