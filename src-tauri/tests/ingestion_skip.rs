use std::path::Path;

use jobs_search_lib::config::{RateLimitConfig, SourceConfig, SourceType};
use jobs_search_lib::db::Db;
use jobs_search_lib::db::jobs::{
    NewJob, add_lineage, get_detail, known_postings, lineage, touch_source_lineage, upsert_job,
};
use jobs_search_lib::dedup::canonical_identity;
use jobs_search_lib::ingestion::adapter::{IngestionRequest, SourceAdapter};
use jobs_search_lib::ingestion::linkedin_mcp::LinkedInMcpAdapter;
use jobs_search_lib::ingestion::normalize;
use jobs_search_lib::types::{JobId, WorkModality};

fn seed_source(db: &Db, id: &str) {
    jobs_search_lib::db::sources::upsert_source(
        db,
        &SourceConfig {
            id: id.to_string(),
            name: id.to_string(),
            source_type: SourceType::Mcp,
            enabled: true,
            config: serde_yaml::from_str("command: python3").expect("config"),
            rate_limit: RateLimitConfig::default(),
            credential_ref: None,
            fallback_for: None,
        },
    )
    .expect("source");
}

fn seed_job(db: &Db, source_id: &str, external_id: &str, description: &str) -> JobId {
    let title = "Rust Engineer";
    let location = "Porto Alegre, Brasil";
    let url = format!("https://www.linkedin.com/jobs/view/{external_id}/");
    let identity = canonical_identity("Acme", title, location, &url);
    let outcome = upsert_job(
        db,
        &NewJob {
            canonical_key: identity.key,
            canonical_hash: identity.hash,
            title: title.to_string(),
            company: "Acme".to_string(),
            location: location.to_string(),
            work_modality: WorkModality::Onsite,
            url: url.clone(),
            posted_at: None,
            description_clean: description.to_string(),
            extracted_metadata_json: None,
            needs_review: identity.used_url_fallback,
        },
    )
    .expect("job");
    add_lineage(db, &outcome.job_id, source_id, external_id, &url).expect("lineage");
    outcome.job_id
}

fn fake_server(log: &Path, ids: &[&str]) -> String {
    let log = log.display().to_string();
    let ids = serde_json::to_string(ids).expect("ids");
    format!(
        r#"
import json
import sys

log = {log:?}
ids = {ids}

for raw in sys.stdin:
    request = json.loads(raw)
    if "id" not in request:
        continue
    request_id = request["id"]
    method = request.get("method")
    if method == "initialize":
        print(json.dumps({{"jsonrpc": "2.0", "id": request_id, "result": {{
            "protocolVersion": "2024-11-05",
            "capabilities": {{}},
            "serverInfo": {{"name": "fake-linkedin", "version": "0"}},
        }}}}), flush=True)
    elif method == "tools/call":
        params = request.get("params") or {{}}
        name = params.get("name")
        if name == "search_jobs":
            payload = {{"job_ids": ids}}
            print(json.dumps({{"jsonrpc": "2.0", "id": request_id, "result": {{
                "content": [{{"type": "text", "text": json.dumps(payload)}}],
                "isError": False,
            }}}}), flush=True)
        elif name == "get_job_details":
            job_id = (params.get("arguments") or {{}}).get("job_id")
            with open(log, "a", encoding="utf-8") as handle:
                handle.write(str(job_id) + "\n")
            body = (
                "Acme\n"
                "Rust Engineer\n"
                "Porto Alegre, Brasil  · Compartilhada há 2 dias · "
                "10 pessoas clicaram\n"
                "Presencial\n"
                "Sobre a vaga\n"
                "We need Rust and Linux for job " + str(job_id) + ".\n"
                "Ative um alerta para vagas semelhantes\n"
            )
            print(json.dumps({{"jsonrpc": "2.0", "id": request_id, "result": {{
                "structuredContent": {{
                    "url": "https://www.linkedin.com/jobs/view/" + str(job_id) + "/",
                    "sections": {{"job_posting": body}},
                }},
                "isError": False,
            }}}}, ensure_ascii=False), flush=True)
        else:
            print(json.dumps({{"jsonrpc": "2.0", "id": request_id, "error": {{
                "code": -32601, "message": "unknown tool"}}}}), flush=True)
"#
    )
}

fn adapter(script: &Path) -> LinkedInMcpAdapter {
    LinkedInMcpAdapter::new(
        "linkedin",
        "linkedin",
        "python3",
        vec![script.display().to_string()],
        10,
        1,
        std::time::Duration::from_secs(5),
    )
}

fn request(known: Vec<jobs_search_lib::ingestion::adapter::KnownPosting>) -> IngestionRequest {
    IngestionRequest {
        query: "Rust Engineer".to_string(),
        location: "Brazil".to_string(),
        results_wanted: 10,
        known_postings: known,
        ..IngestionRequest::default()
    }
}

fn detail_calls(log: &Path) -> Vec<String> {
    match std::fs::read_to_string(log) {
        Ok(contents) => contents
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => panic!("could not read detail log: {error}"),
    }
}

#[tokio::test]
async fn known_postings_skip_details_and_keep_their_description() {
    let directory = tempfile::tempdir().expect("tempdir");
    let log = directory.path().join("details.log");
    let script = directory.path().join("fake.py");
    std::fs::write(&script, fake_server(&log, &["123"])).expect("script");

    let db = Db::open_in_memory().expect("db");
    seed_source(&db, "jobspy");
    seed_source(&db, "linkedin");
    let job_id = seed_job(&db, "jobspy", "linkedin-123", "Stored description");
    db.with_connection(|connection| {
        connection.execute(
            "UPDATE job_posting_sources SET ingested_at = '2000-01-01T00:00:00Z'",
            [],
        )?;
        Ok(())
    })
    .expect("backdate");

    let known = known_postings(&db).expect("known");
    assert_eq!(known.len(), 1);
    let outcome = adapter(&script)
        .fetch(&request(known.clone()))
        .await
        .expect("fetch");

    assert_eq!(outcome.listed, 1);
    assert!(outcome.jobs.is_empty());
    assert_eq!(outcome.known_skipped, vec!["linkedin-123"]);
    assert!(detail_calls(&log).is_empty());

    touch_source_lineage(
        &db,
        &job_id,
        "linkedin",
        "linkedin-123",
        "https://www.linkedin.com/jobs/view/123/",
    )
    .expect("touch");

    let rows = lineage(&db, &job_id).expect("lineage");
    let linkedin = rows
        .iter()
        .find(|row| row.source_id == "linkedin")
        .expect("linkedin lineage");
    assert_ne!(linkedin.ingested_at, "2000-01-01T00:00:00Z");
    assert_eq!(
        get_detail(&db, &job_id).expect("detail").description_clean,
        "Stored description"
    );
}

#[tokio::test]
async fn unknown_and_empty_description_postings_fetch_details() {
    let directory = tempfile::tempdir().expect("tempdir");
    let log = directory.path().join("details.log");
    let script = directory.path().join("fake.py");
    std::fs::write(&script, fake_server(&log, &["123", "456"])).expect("script");

    let db = Db::open_in_memory().expect("db");
    seed_source(&db, "jobspy");
    let empty = seed_job(&db, "jobspy", "linkedin-123", "");

    let known = known_postings(&db).expect("known");
    assert!(known.is_empty());
    let outcome = adapter(&script)
        .fetch(&request(known))
        .await
        .expect("fetch");

    assert_eq!(outcome.listed, 2);
    assert_eq!(outcome.jobs.len(), 2);
    assert!(outcome.known_skipped.is_empty());
    assert_eq!(detail_calls(&log), vec!["123", "456"]);
    assert!(
        outcome
            .jobs
            .iter()
            .all(|job| !job.description.trim().is_empty())
    );

    let raw = outcome
        .jobs
        .into_iter()
        .find(|job| job.external_id == "linkedin-123")
        .expect("known id");
    let normalized = normalize::normalize(raw, "jobspy", "jobspy").expect("normalize");
    upsert_job(
        &db,
        &NewJob {
            canonical_key: normalized.canonical_key,
            canonical_hash: normalized.canonical_hash,
            title: normalized.title,
            company: normalized.company,
            location: normalized.location,
            work_modality: normalized.work_modality,
            url: normalized.url,
            posted_at: normalized.posted_at,
            description_clean: normalized.description_clean,
            extracted_metadata_json: normalized.extracted_metadata_json,
            needs_review: normalized.needs_review,
        },
    )
    .expect("upsert");
    assert!(
        !get_detail(&db, &empty)
            .expect("detail")
            .description_clean
            .is_empty()
    );
}

#[tokio::test]
async fn a_jobspy_owned_known_posting_is_skipped_for_linkedin() {
    let directory = tempfile::tempdir().expect("tempdir");
    let log = directory.path().join("details.log");
    let script = directory.path().join("fake.py");
    std::fs::write(&script, fake_server(&log, &["123"])).expect("script");

    let db = Db::open_in_memory().expect("db");
    seed_source(&db, "jobspy");
    seed_source(&db, "linkedin");
    let job_id = seed_job(&db, "jobspy", "linkedin-123", "Stored by jobspy");

    let known = known_postings(&db).expect("known");
    let outcome = adapter(&script)
        .fetch(&request(known.clone()))
        .await
        .expect("fetch");

    assert!(outcome.jobs.is_empty());
    assert_eq!(outcome.known_skipped, vec!["linkedin-123"]);
    assert!(detail_calls(&log).is_empty());

    let posting = &known[0];
    touch_source_lineage(
        &db,
        &JobId::from(posting.job_id.clone()),
        "linkedin",
        &posting.external_id,
        &posting.url,
    )
    .expect("touch");

    let rows = lineage(&db, &job_id).expect("lineage");
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.iter()
            .filter(|row| row.external_id == "linkedin-123")
            .count(),
        2
    );
    assert_eq!(known_postings(&db).expect("known").len(), 1);
    assert_eq!(
        get_detail(&db, &job_id).expect("detail").description_clean,
        "Stored by jobspy"
    );
}
