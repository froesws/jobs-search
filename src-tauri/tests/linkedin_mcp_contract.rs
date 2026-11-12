use std::time::Duration;

use jobs_search_lib::ingestion::adapter::{IngestionRequest, KnownPosting, SourceAdapter};
use jobs_search_lib::ingestion::linkedin_mcp::LinkedInMcpAdapter;

fn request() -> IngestionRequest {
    IngestionRequest {
        query: "Rust Engineer".to_string(),
        location: "Brazil".to_string(),
        results_wanted: 10,
        ..IngestionRequest::default()
    }
}

fn fake_server() -> String {
    r#"
import json
import sys

def send(value):
    print(json.dumps(value), flush=True)

for raw in sys.stdin:
    request = json.loads(raw)
    if "id" not in request:
        continue
    request_id = request["id"]
    method = request.get("method")
    if method == "initialize":
        send({"jsonrpc": "2.0", "id": request_id, "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "serverInfo": {"name": "fake-linkedin", "version": "0"},
        }})
    elif method == "tools/call":
        name = (request.get("params") or {}).get("name")
        if name == "search_jobs":
            payload = {"job_ids": ["123"], "url": "https://www.linkedin.com/jobs/search/"}
            send({"jsonrpc": "2.0", "id": request_id, "result": {
                "content": [{"type": "text", "text": json.dumps(payload)}],
                "isError": False,
            }})
        elif name == "get_job_details":
            send({"jsonrpc": "2.0", "id": request_id, "result": {
                "structuredContent": {
                    "url": "https://www.linkedin.com/jobs/view/123/",
                    "sections": {"job_posting": (
                        "Acme\n"
                        "Rust Engineer\n"
                        "Porto Alegre, Brasil  · Compartilhada há 2 dias · "
                        "10 pessoas clicaram\n"
                        "Presencial\n"
                        "Sobre a vaga\n"
                        "We need Rust and Linux.\n"
                        "Ative um alerta para vagas semelhantes\n"
                        "Mais vagas\n"
                        "Other job"
                    )},
                },
                "isError": False,
            }})
        else:
            send({"jsonrpc": "2.0", "id": request_id, "error": {
                "code": -32601, "message": "unknown tool"}})
    else:
        send({"jsonrpc": "2.0", "id": request_id, "error": {
            "code": -32601, "message": "unknown method"}})
"#
    .to_string()
}

#[tokio::test]
async fn linkedin_mcp_adapter_reads_search_and_details() {
    let directory = tempfile::tempdir().expect("tempdir");
    let script = directory.path().join("fake_linkedin_mcp.py");
    std::fs::write(&script, fake_server()).expect("script");

    let adapter = LinkedInMcpAdapter::new(
        "linkedin",
        "linkedin",
        "python3",
        vec![script.display().to_string()],
        10,
        1,
        Duration::from_secs(5),
    );

    let jobs = adapter.fetch(&request()).await.expect("jobs").jobs;
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].title, "Rust Engineer");
    assert_eq!(jobs[0].company, "Acme");
    assert_eq!(jobs[0].location, "Porto Alegre, Brasil");
    assert_eq!(jobs[0].work_modality.as_deref(), Some("Onsite"));
    assert_eq!(jobs[0].external_id, "linkedin-123");
    assert!(jobs[0].description.contains("Rust and Linux"));
    assert!(!jobs[0].description.contains("Other job"));
}

#[tokio::test]
async fn linkedin_mcp_adapter_runs_each_query_variant() {
    let directory = tempfile::tempdir().expect("tempdir");
    let script = directory.path().join("variant_linkedin_mcp.py");
    let log = directory.path().join("queries.log");
    let source = format!(
        r#"
import json
import sys

log = {log:?}
ids = {{
    "Rust Engineer trainee": "123",
    "Rust Engineer júnior": "456",
}}

for raw in sys.stdin:
    request = json.loads(raw)
    if "id" not in request:
        continue
    request_id = request["id"]
    if request["method"] == "initialize":
        print(json.dumps({{"jsonrpc": "2.0", "id": request_id, "result": {{}}}}), flush=True)
    elif request["method"] == "tools/call":
        params = request.get("params") or {{}}
        name = params.get("name")
        arguments = params.get("arguments") or {{}}
        if name == "search_jobs":
            query = arguments.get("keywords", "")
            with open(log, "a") as handle:
                handle.write(query + "\n")
            job_id = ids.get(query, "999")
            payload = {{"job_ids": [job_id], "url": "https://www.linkedin.com/jobs/search/"}}
            print(json.dumps({{
                "jsonrpc": "2.0",
                "id": request_id,
                "result": {{
                    "content": [{{"type": "text", "text": json.dumps(payload)}}],
                    "isError": False,
                }},
            }}), flush=True)
        elif name == "get_job_details":
            job_id = arguments.get("job_id", "999")
            posting = (
                "Acme\n"
                f"Rust Engineer {{job_id}}\n"
                "Porto Alegre, Brasil  · Compartilhada há 2 dias · 10 pessoas clicaram\n"
                "Presencial\n"
                "Sobre a vaga\n"
                "We need Rust and Linux.\n"
                "Ative um alerta para vagas semelhantes\n"
            )
            print(json.dumps({{
                "jsonrpc": "2.0",
                "id": request_id,
                "result": {{
                    "structuredContent": {{
                        "url": f"https://www.linkedin.com/jobs/view/{{job_id}}/",
                        "sections": {{"job_posting": posting}},
                    }},
                    "isError": False,
                }},
            }}), flush=True)
        else:
            print(json.dumps({{"jsonrpc": "2.0", "id": request_id, "error": {{
                "code": -32601, "message": "unknown tool"}}}}), flush=True)
"#
    );
    std::fs::write(&script, source).expect("script");

    let adapter = LinkedInMcpAdapter::new(
        "linkedin",
        "linkedin",
        "python3",
        vec![script.display().to_string()],
        10,
        1,
        Duration::from_secs(5),
    );
    let request = IngestionRequest {
        queries: vec![
            "Rust Engineer trainee".to_string(),
            "Rust Engineer júnior".to_string(),
        ],
        results_wanted: 10,
        ..IngestionRequest::default()
    };

    let jobs = adapter.fetch(&request).await.expect("jobs").jobs;
    assert_eq!(jobs.len(), 2);
    let written = std::fs::read_to_string(log).expect("log");
    assert_eq!(
        written.lines().collect::<Vec<_>>(),
        ["Rust Engineer trainee", "Rust Engineer júnior"]
    );
}

#[tokio::test]
async fn linkedin_children_do_not_inherit_virtual_env() {
    let directory = tempfile::tempdir().expect("tempdir");
    let script = directory.path().join("env_linkedin_mcp.py");
    let log = directory.path().join("env.txt");
    std::fs::write(
        &script,
        format!(
            r#"
import json
import os
import pathlib
import sys

pathlib.Path({log:?}).write_text(os.environ.get("VIRTUAL_ENV") or "unset")

def send(value):
    print(json.dumps(value), flush=True)

for raw in sys.stdin:
    request = json.loads(raw)
    if "id" not in request:
        continue
    request_id = request["id"]
    if request["method"] == "initialize":
        send({{"jsonrpc": "2.0", "id": request_id, "result": {{
            "protocolVersion": "2024-11-05",
            "capabilities": {{}},
            "serverInfo": {{"name": "fake-linkedin", "version": "0"}},
        }}}})
    elif request["method"] == "tools/call":
        name = (request.get("params") or {{}}).get("name")
        if name == "search_jobs":
            payload = {{"job_ids": []}}
            send({{"jsonrpc": "2.0", "id": request_id, "result": {{
                "content": [{{"type": "text", "text": json.dumps(payload)}}],
                "isError": False,
            }}}})
        else:
            send({{"jsonrpc": "2.0", "id": request_id, "error": {{
                "code": -32601, "message": "unknown tool"}}}})
"#
        ),
    )
    .expect("script");

    static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    struct EnvGuard;
    impl EnvGuard {
        fn set(value: &str) -> Self {
            // Tests that mutate the process environment must serialize.
            unsafe { std::env::set_var("VIRTUAL_ENV", value) };
            Self
        }
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe { std::env::remove_var("VIRTUAL_ENV") };
        }
    }

    let _lock = ENV_LOCK.lock().await;
    let _guard = EnvGuard::set(".venv");

    let adapter = LinkedInMcpAdapter::new(
        "linkedin",
        "linkedin",
        "python3",
        vec![script.display().to_string()],
        10,
        1,
        Duration::from_secs(5),
    );

    adapter.fetch(&request()).await.expect("jobs");
    assert_eq!(std::fs::read_to_string(&log).expect("env log"), "unset");
}

#[tokio::test]
async fn known_ids_skip_get_job_details() {
    let directory = tempfile::tempdir().expect("tempdir");
    let script = directory.path().join("count_details.py");
    let log = directory.path().join("details.log");
    let source = format!(
        r#"
import json
import sys

log = {log:?}

for raw in sys.stdin:
    request = json.loads(raw)
    if "id" not in request:
        continue
    request_id = request["id"]
    if request["method"] == "initialize":
        print(json.dumps({{"jsonrpc": "2.0", "id": request_id, "result": {{
            "protocolVersion": "2024-11-05",
            "capabilities": {{}},
            "serverInfo": {{"name": "fake-linkedin", "version": "0"}},
        }}}}), flush=True)
    elif request["method"] == "tools/call":
        name = (request.get("params") or {{}}).get("name")
        if name == "search_jobs":
            payload = {{"job_ids": ["123", "456"]}}
            print(json.dumps({{"jsonrpc": "2.0", "id": request_id, "result": {{
                "content": [{{"type": "text", "text": json.dumps(payload)}}],
                "isError": False,
            }}}}), flush=True)
        elif name == "get_job_details":
            job_id = (request["params"].get("arguments") or {{}}).get("job_id")
            with open(log, "a", encoding="utf-8") as handle:
                handle.write(str(job_id) + "\n")
            body = (
                "Acme\nRust Engineer\nPorto Alegre, Brasil\nPresencial\n"
                "Sobre a vaga\nWe need Rust and Linux.\n"
            )
            print(json.dumps({{"jsonrpc": "2.0", "id": request_id, "result": {{
                "structuredContent": {{
                    "url": "https://www.linkedin.com/jobs/view/" + str(job_id) + "/",
                    "sections": {{"job_posting": body}},
                }},
                "isError": False,
            }}}}, ensure_ascii=False), flush=True)
"#,
    );
    std::fs::write(&script, source).expect("script");
    let adapter = LinkedInMcpAdapter::new(
        "linkedin",
        "linkedin",
        "python3",
        vec![script.display().to_string()],
        10,
        1,
        Duration::from_secs(5),
    );

    let known = IngestionRequest {
        known_postings: vec![KnownPosting {
            external_id: "linkedin-123".to_string(),
            job_id: "job-1".to_string(),
            url: "https://www.linkedin.com/jobs/view/123/".to_string(),
        }],
        ..request()
    };
    let outcome = adapter.fetch(&known).await.expect("known fetch");
    assert_eq!(outcome.listed, 2);
    assert_eq!(outcome.jobs.len(), 1);
    assert_eq!(outcome.known_skipped, vec!["linkedin-123"]);
    let calls = std::fs::read_to_string(&log).expect("log");
    assert_eq!(
        calls.lines().collect::<Vec<_>>(),
        ["456"],
        "the known ID must not request details"
    );

    let unknown = IngestionRequest {
        known_postings: Vec::new(),
        ..request()
    };
    let outcome = adapter.fetch(&unknown).await.expect("unknown fetch");
    assert_eq!(outcome.jobs.len(), 2);
    assert!(outcome.known_skipped.is_empty());
    let calls = std::fs::read_to_string(&log).expect("log");
    assert_eq!(calls.lines().collect::<Vec<_>>(), ["456", "123", "456"]);
}
