use jobs_search_lib::db::jobs::StoredJob;
use jobs_search_lib::matching::filters::{SearchCriteria, check, check_with_criteria};
use jobs_search_lib::profile::CandidateProfile;
use jobs_search_lib::types::{Seniority, WorkModality};

mod common;

fn profile() -> CandidateProfile {
    common::fixture_profile()
}

fn job(title: &str, location: &str, modality: WorkModality, description: &str) -> StoredJob {
    StoredJob {
        id: jobs_search_lib::types::JobId::from("job"),
        title: title.to_string(),
        company: "Acme".to_string(),
        location: location.to_string(),
        work_modality: modality,
        url: "https://example.com/1".to_string(),
        posted_at: None,
        description_clean: description.to_string(),
        extracted_metadata_json: None,
    }
}

#[test]
fn remote_brazil_and_accepted_cities_pass() {
    assert!(
        check(
            &profile(),
            &job(
                "Rust Engineer",
                "Remote - Brazil",
                WorkModality::Remote,
                "Rust"
            )
        )
        .passed
    );
    assert!(
        check(
            &profile(),
            &job(
                "Rust Engineer",
                "Porto Alegre, RS",
                WorkModality::Onsite,
                "Rust"
            )
        )
        .passed
    );
    assert!(
        check(
            &profile(),
            &job("Rust Engineer", "Curitiba", WorkModality::Hybrid, "Rust")
        )
        .passed
    );
}

#[test]
fn global_remote_is_accepted_but_foreign_restrictions_are_not() {
    assert!(
        check(
            &profile(),
            &job(
                "Rust Engineer",
                "Remote - Worldwide",
                WorkModality::Remote,
                "Rust"
            )
        )
        .passed
    );
    assert!(
        !check(
            &profile(),
            &job(
                "Rust Engineer",
                "Remote - USA only",
                WorkModality::Remote,
                "Rust"
            )
        )
        .passed
    );
}

#[test]
fn onsite_outside_cities_is_rejected() {
    let result = check(
        &profile(),
        &job("Rust Engineer", "São Paulo", WorkModality::Onsite, "Rust"),
    );
    assert!(!result.passed);
    assert!(
        result
            .rejections
            .iter()
            .any(|reason| reason.code == "LOCATION_NOT_ELIGIBLE")
    );
}

#[test]
fn senior_only_is_rejected_and_pleno_senior_range_is_accepted() {
    let senior = check(
        &profile(),
        &job(
            "Senior Rust Engineer",
            "Remote - Brazil",
            WorkModality::Remote,
            "Rust",
        ),
    );
    assert!(!senior.passed);
    assert!(
        senior
            .rejections
            .iter()
            .any(|reason| reason.code == "SENIORITY_NOT_ELIGIBLE")
    );

    assert!(
        check(
            &profile(),
            &job(
                "Pleno/Sênior Rust Engineer",
                "Remote - Brazil",
                WorkModality::Remote,
                "Rust",
            ),
        )
        .passed
    );
}

#[test]
fn selected_seniorities_override_the_profile_and_allow_multiple_levels() {
    let criteria = SearchCriteria {
        seniorities: vec![Seniority::Trainee, Seniority::Junior, Seniority::Pleno],
        ..SearchCriteria::default()
    };

    assert!(
        check_with_criteria(
            &profile(),
            &job(
                "Pleno/Sênior Rust Engineer",
                "Remote - Brazil",
                WorkModality::Remote,
                "Rust",
            ),
            &criteria,
        )
        .passed
    );
    assert!(
        !check_with_criteria(
            &profile(),
            &job(
                "Senior Rust Engineer",
                "Remote - Brazil",
                WorkModality::Remote,
                "Rust",
            ),
            &criteria,
        )
        .passed
    );

    let pleno_senior = SearchCriteria {
        seniorities: vec![Seniority::Pleno, Seniority::Senior],
        ..SearchCriteria::default()
    };
    assert!(
        check_with_criteria(
            &profile(),
            &job(
                "Senior Rust Engineer",
                "Remote - Brazil",
                WorkModality::Remote,
                "Rust",
            ),
            &pleno_senior,
        )
        .passed
    );
}

#[test]
fn missing_core_technology_is_rejected_except_for_trainee() {
    let missing = check(
        &profile(),
        &job(
            "Marketing Analyst",
            "Remote - Brazil",
            WorkModality::Remote,
            "marketing",
        ),
    );
    assert!(!missing.passed);
    assert!(
        missing
            .rejections
            .iter()
            .any(|reason| reason.code == "NO_CORE_TECHNOLOGY")
    );

    assert!(
        check(
            &profile(),
            &job(
                "Trainee Generalist",
                "Remote - Brazil",
                WorkModality::Remote,
                "generalist",
            ),
        )
        .passed
    );
}

#[test]
fn metadata_only_postings_are_not_rejected_for_missing_technology() {
    assert!(
        check(
            &profile(),
            &job(
                "Analista de Desenvolvimento de Software Pleno",
                "Brazil",
                WorkModality::Unknown,
                "",
            ),
        )
        .passed
    );
}

#[test]
fn dealbreakers_are_rejected() {
    let result = check(
        &profile(),
        &job(
            "Rust Engineer",
            "Remote - Brazil",
            WorkModality::Remote,
            "commission only position",
        ),
    );
    assert!(!result.passed);
    assert!(
        result
            .rejections
            .iter()
            .any(|reason| reason.code == "DEALBREAKER")
    );
}

#[test]
fn accepted_seniorities_come_from_the_profile() {
    let accepted = profile().accepted_seniorities();
    assert!(accepted.contains(&Seniority::Trainee));
    assert!(accepted.contains(&Seniority::Junior));
    assert!(accepted.contains(&Seniority::Pleno));
    assert!(!accepted.contains(&Seniority::Senior));
}

#[test]
fn unknown_mandatory_values_do_not_reject_but_missing_domains_do() {
    let mut profile = profile();
    profile.mandatory_criteria = vec![
        "location".to_string(),
        "modality".to_string(),
        "seniority".to_string(),
        "domain:Kubernetes".to_string(),
    ];

    let unknown = check(
        &profile,
        &job(
            "Rust Engineer",
            "Remote - Brazil",
            WorkModality::Unknown,
            "Rust and Kubernetes",
        ),
    );
    assert!(
        unknown.passed,
        "unknown modality must not reject: {unknown:?}"
    );

    let unknown_location = check(
        &profile,
        &job(
            "Rust Engineer",
            "Unknown",
            WorkModality::Remote,
            "Rust and Kubernetes",
        ),
    );
    assert!(
        unknown_location.passed,
        "unknown location must not reject: {unknown_location:?}"
    );

    let missing_domain = check(
        &profile,
        &job(
            "Pleno Rust Engineer",
            "Remote - Brazil",
            WorkModality::Remote,
            "Rust",
        ),
    );
    assert!(!missing_domain.passed);
    assert!(
        missing_domain
            .rejections
            .iter()
            .any(|reason| reason.code == "UNKNOWN_MANDATORY_CRITERION")
    );

    let known = check(
        &profile,
        &job(
            "Pleno Rust Engineer",
            "Remote - Brazil",
            WorkModality::Remote,
            "Rust and Kubernetes",
        ),
    );
    assert!(known.passed);
}

#[test]
fn selected_modalities_are_enforced_with_city_rules() {
    let remote_only = SearchCriteria {
        modalities: vec![WorkModality::Remote],
        ..SearchCriteria::default()
    };
    assert!(
        check_with_criteria(
            &profile(),
            &job(
                "Rust Engineer",
                "Remote - Brazil",
                WorkModality::Remote,
                "Rust",
            ),
            &remote_only,
        )
        .passed
    );
    for (title, location, modality) in [
        ("Rust Engineer", "Porto Alegre", WorkModality::Hybrid),
        ("Rust Engineer", "Curitiba", WorkModality::Onsite),
    ] {
        let result = check_with_criteria(
            &profile(),
            &job(title, location, modality, "Rust"),
            &remote_only,
        );
        assert!(!result.passed);
        assert!(
            result
                .rejections
                .iter()
                .any(|reason| reason.code == "MODALITY_NOT_ELIGIBLE")
        );
    }

    let hybrid_only = SearchCriteria {
        modalities: vec![WorkModality::Hybrid],
        ..SearchCriteria::default()
    };
    assert!(
        check_with_criteria(
            &profile(),
            &job(
                "Rust Engineer",
                "Porto Alegre",
                WorkModality::Hybrid,
                "Rust"
            ),
            &hybrid_only,
        )
        .passed
    );
    assert!(
        !check_with_criteria(
            &profile(),
            &job(
                "Rust Engineer",
                "Remote - Brazil",
                WorkModality::Remote,
                "Rust"
            ),
            &hybrid_only,
        )
        .passed
    );

    let onsite_only = SearchCriteria {
        modalities: vec![WorkModality::Onsite],
        ..SearchCriteria::default()
    };
    assert!(
        check_with_criteria(
            &profile(),
            &job("Rust Engineer", "Curitiba", WorkModality::Onsite, "Rust"),
            &onsite_only,
        )
        .passed
    );
    let sao_paulo = check_with_criteria(
        &profile(),
        &job("Rust Engineer", "São Paulo", WorkModality::Onsite, "Rust"),
        &onsite_only,
    );
    assert!(!sao_paulo.passed);
    assert!(
        sao_paulo
            .rejections
            .iter()
            .any(|reason| reason.code == "LOCATION_NOT_ELIGIBLE")
    );

    let mut open_profile = profile();
    open_profile.mandatory_criteria.clear();
    let unknown = job(
        "Rust Engineer",
        "Porto Alegre",
        WorkModality::Unknown,
        "Rust",
    );
    let restricted = check_with_criteria(&open_profile, &unknown, &remote_only);
    assert!(
        restricted
            .rejections
            .iter()
            .any(|reason| reason.code == "MODALITY_NOT_ELIGIBLE")
    );
    let all_modalities = SearchCriteria {
        modalities: vec![
            WorkModality::Remote,
            WorkModality::Hybrid,
            WorkModality::Onsite,
        ],
        ..SearchCriteria::default()
    };
    assert!(check_with_criteria(&open_profile, &unknown, &all_modalities).passed);
}
