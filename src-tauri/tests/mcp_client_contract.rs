use std::time::Duration;

use jobs_search_lib::ingestion::adapter::{IngestionRequest, SourceAdapter};
use jobs_search_lib::ingestion::mcp::McpAdapter;

mod common;

fn request() -> IngestionRequest {
    IngestionRequest {
        query: "Rust Engineer".to_string(),
        stack_keyword: "Rust".to_string(),
        ..IngestionRequest::default()
    }
}

#[tokio::test]
async fn stdio_client_initializes_and_reads_tool_result() {
    let directory = tempfile::tempdir().expect("tempdir");
    let script = common::write_fake_mcp_server(directory.path());
    let adapter = McpAdapter::new(
        "jobspy",
        "jobspy",
        "python3",
        vec![script.display().to_string()],
        vec!["linkedin".to_string(), "indeed".to_string()],
        Duration::from_secs(5),
    );

    let jobs = adapter.fetch(&request()).await.expect("jobs");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].title, "Rust Engineer");
    assert_eq!(jobs[0].company, "Acme");
    assert_eq!(jobs[0].location, "Remote - Brazil");
    assert_eq!(jobs[0].external_id, "li-1");
    assert!(jobs[0].description.contains("Rust systems"));
}

#[tokio::test]
async fn stdio_client_requests_descriptions_by_default() {
    use jobs_search_lib::config::SourceConfig;
    use std::path::Path;

    let directory = tempfile::tempdir().expect("tempdir");
    let script = directory.path().join("echo_args.py");
    std::fs::write(
        &script,
        r#"
import json
import sys
for raw in sys.stdin:
    request = json.loads(raw)
    if "id" not in request:
        continue
    if request["method"] == "initialize":
        result = {}
    elif request["method"] == "tools/call":
        value = request["params"]["arguments"].get("fetch_descriptions")
        result = {"jobs": [{"title": f"fetch={value!r}", "url": "https://example.com/1"}]}
    print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}), flush=True)
"#,
    )
    .expect("script");

    let default_adapter = McpAdapter::new(
        "jobspy",
        "jobspy",
        "python3",
        vec![script.display().to_string()],
        Vec::new(),
        Duration::from_secs(5),
    );
    let jobs = default_adapter.fetch(&request()).await.expect("jobs");
    assert_eq!(jobs[0].title, "fetch=True");

    let source: SourceConfig = serde_yaml::from_str(&format!(
        r#"
id: jobspy
name: jobspy
source_type: mcp
config:
  command: python3
  args: ["{}"]
  fetch_descriptions: false
"#,
        script.display()
    ))
    .expect("source");
    let disabled_adapter = McpAdapter::from_config(&source, Path::new("/tmp")).expect("adapter");
    let jobs = disabled_adapter.fetch(&request()).await.expect("jobs");
    assert_eq!(jobs[0].title, "fetch=False");
}

#[tokio::test]
async fn children_do_not_inherit_virtual_env() {
    let directory = tempfile::tempdir().expect("tempdir");
    let script = directory.path().join("env.py");
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

for raw in sys.stdin:
    request = json.loads(raw)
    if "id" not in request:
        continue
    if request["method"] == "initialize":
        result = {{}}
    else:
        result = {{"jobs": [{{"title": "Rust", "url": "https://example.com/1"}}]}}
    print(json.dumps({{"jsonrpc": "2.0", "id": request["id"], "result": result}}), flush=True)
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

    let adapter = McpAdapter::new(
        "jobspy",
        "jobspy",
        "python3",
        vec![script.display().to_string()],
        Vec::new(),
        Duration::from_secs(5),
    );
    adapter.fetch(&request()).await.expect("jobs");

    assert_eq!(std::fs::read_to_string(&log).expect("env log"), "unset");
}

#[tokio::test]
async fn stdio_timeout_is_isolated_as_a_source_error() {
    let directory = tempfile::tempdir().expect("tempdir");
    let script = directory.path().join("slow.py");
    std::fs::write(
        &script,
        "import sys, time\nfor line in sys.stdin:\n    time.sleep(2)\n",
    )
    .expect("script");
    let adapter = McpAdapter::new(
        "jobspy",
        "jobspy",
        "python3",
        vec![script.display().to_string()],
        Vec::new(),
        Duration::from_millis(120),
    );

    let error = adapter.fetch(&request()).await.expect_err("timeout");
    assert_eq!(error.code, "MCP_TIMEOUT");
    assert!(error.retryable);
}

#[tokio::test]
async fn stdio_tool_error_is_sanitized() {
    let directory = tempfile::tempdir().expect("tempdir");
    let script = directory.path().join("error.py");
    std::fs::write(
        &script,
        r#"
import json
import sys
for raw in sys.stdin:
    request = json.loads(raw)
    if "id" not in request:
        continue
    if request["method"] == "initialize":
        response = {"jsonrpc": "2.0", "id": request["id"], "result": {}}
    else:
        response = {"jsonrpc": "2.0", "id": request["id"], "error": {"code": -32000, "message": "rate limited"}}
    print(json.dumps(response), flush=True)
"#,
    )
    .expect("script");
    let adapter = McpAdapter::new(
        "jobspy",
        "jobspy",
        "python3",
        vec![script.display().to_string()],
        Vec::new(),
        Duration::from_secs(5),
    );

    let error = adapter.fetch(&request()).await.expect_err("tool error");
    assert_eq!(error.code, "MCP_TOOL_ERROR");
    assert_eq!(error.message, "rate limited");
}

#[tokio::test]
async fn missing_command_is_a_permanent_source_error() {
    let adapter = McpAdapter::new(
        "jobspy",
        "jobspy",
        "/definitely/not/a/command",
        Vec::new(),
        Vec::new(),
        Duration::from_secs(1),
    );
    let error = adapter.fetch(&request()).await.expect_err("start failure");
    assert_eq!(error.code, "MCP_START_FAILED");
    assert!(!error.retryable);
}

#[tokio::test]
async fn malformed_json_rpc_is_a_source_error() {
    let directory = tempfile::tempdir().expect("tempdir");
    let script = directory.path().join("malformed.py");
    std::fs::write(
        &script,
        r#"
import json
import sys
for raw in sys.stdin:
    request = json.loads(raw)
    if "id" not in request:
        continue
    if request["method"] == "initialize":
        print(json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": {}}), flush=True)
    else:
        print("not json", flush=True)
"#,
    )
    .expect("script");
    let adapter = McpAdapter::new(
        "jobspy",
        "jobspy",
        "python3",
        vec![script.display().to_string()],
        Vec::new(),
        Duration::from_secs(5),
    );

    let error = adapter.fetch(&request()).await.expect_err("malformed");
    assert_eq!(error.code, "MCP_INVALID_RESPONSE");
    assert!(error.retryable);
}

#[tokio::test]
async fn stdio_client_runs_each_query_variant() {
    let directory = tempfile::tempdir().expect("tempdir");
    let script = directory.path().join("variants.py");
    let log = directory.path().join("queries.log");
    let source = format!(
        r#"
import json
import os
import sys

log = {log:?}

for raw in sys.stdin:
    request = json.loads(raw)
    if "id" not in request:
        continue
    if request["method"] == "initialize":
        print(json.dumps({{"jsonrpc": "2.0", "id": request["id"], "result": {{}}}}), flush=True)
    elif request["method"] == "tools/call":
        arguments = request["params"]["arguments"]
        query = arguments["query"]
        with open(log, "a") as handle:
            handle.write(query + "\n")
        print(json.dumps({{
            "jsonrpc": "2.0",
            "id": request["id"],
            "result": {{
                "jobs": [{{
                    "title": query,
                    "company": "Acme",
                    "location": "Remote - Brazil",
                    "url": "https://example.com/" + query.replace(" ", "-"),
                    "description": "Rust",
                }}]
            }}
        }}), flush=True)
"#
    );
    std::fs::write(&script, source).expect("script");
    let adapter = McpAdapter::new(
        "jobspy",
        "jobspy",
        "python3",
        vec![script.display().to_string()],
        Vec::new(),
        Duration::from_secs(5),
    );
    let request = IngestionRequest {
        queries: vec![
            "Rust Engineer trainee".to_string(),
            "Rust Engineer júnior".to_string(),
            "Rust Engineer pleno".to_string(),
        ],
        ..IngestionRequest::default()
    };

    let jobs = adapter.fetch(&request).await.expect("jobs");
    assert_eq!(jobs.len(), 3);
    let written = std::fs::read_to_string(log).expect("log");
    assert_eq!(
        written.lines().collect::<Vec<_>>(),
        [
            "Rust Engineer trainee",
            "Rust Engineer júnior",
            "Rust Engineer pleno"
        ]
    );
}
