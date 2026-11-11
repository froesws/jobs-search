use std::time::Duration;

use jobs_search_lib::ingestion::adapter::{IngestionRequest, SourceAdapter};
use jobs_search_lib::ingestion::mcp::McpAdapter;

fn request() -> IngestionRequest {
    IngestionRequest {
        query: "Rust Engineer".to_string(),
        stack_keyword: "Rust".to_string(),
        ..IngestionRequest::default()
    }
}

#[tokio::test]
async fn stdio_exit_status_is_reported() {
    let directory = tempfile::tempdir().expect("tempdir");
    let script = directory.path().join("signal.py");
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
        sys.exit(143)
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

    let error = adapter.fetch(&request()).await.expect_err("exit");
    assert_eq!(error.code, "MCP_PIPE_FAILED");
    assert!(
        error.message.contains("status=143"),
        "message should include the exit code: {}",
        error.message
    );
}
