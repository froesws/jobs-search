//! JobSpy MCP stdio client.
#![allow(missing_docs)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use zeroize::Zeroizing;

use crate::config::SourceConfig;
use crate::error::AppError;
use crate::ingestion::SourcePacer;
use crate::ingestion::adapter::{IngestionRequest, RawJob, SourceAdapter, SourceError};

const PROTOCOL_VERSION: &str = "2024-11-05";
const MAX_RESPONSE_LINES: usize = 20;

pub struct McpAdapter {
    id: String,
    name: String,
    command: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    sites: Vec<String>,
    results_wanted: u32,
    fetch_descriptions: bool,
    timeout: Duration,
    pacer: SourcePacer,
}

impl McpAdapter {
    pub fn from_config(source: &SourceConfig, base_dir: &Path) -> Result<Self, AppError> {
        let command = source
            .config_str("command")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::Config(format!("mcp source {} requires config.command", source.id))
            })?
            .to_string();

        let sites = {
            let configured = source.config_string_list("sites");
            if configured.is_empty() {
                vec!["linkedin".to_string(), "indeed".to_string()]
            } else {
                configured
            }
        };

        Ok(Self {
            id: source.id.clone(),
            name: source.name.clone(),
            command,
            args: source.config_string_list("args"),
            cwd: source.resolved_cwd(base_dir),
            sites,
            results_wanted: source.config_u32("results_wanted", 50).clamp(1, 200),
            fetch_descriptions: source.config_bool("fetch_descriptions", true),
            timeout: Duration::from_secs(source.rate_limit.timeout_seconds.clamp(1, 300)),
            pacer: SourcePacer::new(source.rate_limit.requests_per_minute),
        })
    }

    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
        sites: Vec<String>,
        timeout: Duration,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            command: command.into(),
            args,
            cwd: None,
            sites,
            results_wanted: 50,
            fetch_descriptions: true,
            timeout,
            pacer: SourcePacer::new(60_000),
        }
    }
}

#[async_trait]
impl SourceAdapter for McpAdapter {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn source_type(&self) -> &'static str {
        "mcp"
    }

    async fn fetch(&self, request: &IngestionRequest) -> Result<Vec<RawJob>, SourceError> {
        self.pacer.wait().await;
        let mut command = Command::new(&self.command);
        command
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .env_remove("VIRTUAL_ENV");
        if let Some(cwd) = &self.cwd {
            command.current_dir(cwd);
        }
        let mut child = command.spawn().map_err(|_| {
            SourceError::permanent("MCP_START_FAILED", "could not start the MCP sidecar")
        })?;

        let mut stdin = child.stdin.take().ok_or_else(|| {
            SourceError::permanent("MCP_PIPE_FAILED", "sidecar stdin is unavailable")
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            SourceError::permanent("MCP_PIPE_FAILED", "sidecar stdout is unavailable")
        })?;
        let mut reader = BufReader::new(stdout);

        let result = self.exchange(request, &mut stdin, &mut reader).await;
        let _ = child.kill().await;
        let _ = child.wait().await;
        result
    }
}

impl McpAdapter {
    async fn exchange(
        &self,
        request: &IngestionRequest,
        stdin: &mut ChildStdin,
        reader: &mut BufReader<tokio::process::ChildStdout>,
    ) -> Result<Vec<RawJob>, SourceError> {
        write_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": { "name": "jobs-search", "version": "0.1.0" }
                }
            }),
        )
        .await?;

        let initialize = read_message(reader, self.timeout).await?;
        if let Some(error) = initialize.get("error") {
            return Err(SourceError::transient(
                "MCP_INIT_FAILED",
                jsonrpc_error_message(error),
            ));
        }

        write_message(
            stdin,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized"
            }),
        )
        .await?;

        let sites = if request.sites.is_empty() {
            self.sites.clone()
        } else {
            request.sites.clone()
        };
        let results_wanted = if request.results_wanted == 0 {
            self.results_wanted
        } else {
            request.results_wanted
        };
        let queries = if request.queries.is_empty() {
            vec![request.query.clone()]
        } else {
            request.queries.clone()
        };

        let mut jobs = Vec::new();
        for (index, query) in queries.iter().enumerate() {
            let call_id = format!("jobs-search-search-{}", index + 1);
            write_message(
                stdin,
                &serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": call_id,
                    "method": "tools/call",
                    "params": {
                        "name": "search_jobs",
                        "arguments": {
                            "query": query,
                            "location": request.location,
                            "sites": sites,
                            "results_wanted": results_wanted,
                            "hours_old": request.hours_old,
                            "is_remote": request.is_remote,
                            "fetch_descriptions": self.fetch_descriptions,
                            "job_type": null,
                            "offset": request.offset
                        }
                    }
                }),
            )
            .await?;

            let mut received = false;
            for _ in 0..MAX_RESPONSE_LINES {
                let message = read_message(reader, self.timeout).await?;
                if message.get("id").and_then(serde_json::Value::as_str) != Some(call_id.as_str()) {
                    continue;
                }
                if let Some(error) = message.get("error") {
                    return Err(SourceError::transient(
                        "MCP_TOOL_ERROR",
                        jsonrpc_error_message(error),
                    ));
                }
                let result = message.get("result").ok_or_else(|| {
                    SourceError::transient("MCP_INVALID_RESPONSE", "sidecar response has no result")
                })?;
                let mut batch = self.convert_jobs(result)?;
                if let Some(failures) = result.get("failures").and_then(serde_json::Value::as_array)
                {
                    tracing::warn!(
                        source = %self.id,
                        failed_sites = failures.len(),
                        "some MCP sites failed"
                    );
                }
                jobs.append(&mut batch);
                received = true;
                break;
            }
            if !received {
                return Err(SourceError::transient(
                    "MCP_INVALID_RESPONSE",
                    "sidecar did not return the requested tool result",
                ));
            }
        }

        let mut seen = HashSet::new();
        jobs.retain(|job| seen.insert(job.external_id.clone()));
        Ok(jobs)
    }

    fn convert_jobs(&self, result: &serde_json::Value) -> Result<Vec<RawJob>, SourceError> {
        let jobs_value = if let Some(jobs) = result.get("jobs") {
            jobs.clone()
        } else if let Some(content) = result
            .get("content")
            .and_then(serde_json::Value::as_array)
            .and_then(|entries| entries.first())
            .and_then(|entry| entry.get("text"))
            .and_then(serde_json::Value::as_str)
        {
            let parsed: serde_json::Value = serde_json::from_str(content).map_err(|_| {
                SourceError::transient("MCP_INVALID_RESPONSE", "sidecar content is not valid JSON")
            })?;
            parsed.get("jobs").cloned().ok_or_else(|| {
                SourceError::transient("MCP_INVALID_RESPONSE", "sidecar content has no jobs array")
            })?
        } else {
            return Err(SourceError::transient(
                "MCP_INVALID_RESPONSE",
                "sidecar result has no jobs array",
            ));
        };

        let jobs: Vec<McpJob> = serde_json::from_value(jobs_value).map_err(|_| {
            SourceError::transient("MCP_INVALID_RESPONSE", "sidecar job payload is invalid")
        })?;

        jobs.into_iter()
            .map(|job| {
                let url = job.url.ok_or_else(|| {
                    SourceError::transient("MCP_INVALID_RESPONSE", "sidecar job has no url")
                })?;
                let external_id = job
                    .external_id
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| crate::dedup::identity::sha256_hex(&url));
                let description = job
                    .description
                    .or(job.description_clean)
                    .unwrap_or_default();
                Ok(RawJob {
                    external_id,
                    title: job.title,
                    company: job.company.unwrap_or_else(|| "Unknown".to_string()),
                    location: job.location.unwrap_or_else(|| "Unknown".to_string()),
                    work_modality: job.work_modality,
                    url,
                    posted_at: job.posted_at,
                    description: Zeroizing::new(description),
                    source_name: job.source_name.unwrap_or_else(|| self.name.clone()),
                    tags: job.tags,
                })
            })
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct McpJob {
    title: String,
    #[serde(default)]
    company: Option<String>,
    #[serde(default)]
    location: Option<String>,
    #[serde(default)]
    work_modality: Option<String>,
    #[serde(default, alias = "job_url", alias = "apply_url")]
    url: Option<String>,
    #[serde(default)]
    posted_at: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    description_clean: Option<String>,
    #[serde(default)]
    source_name: Option<String>,
    #[serde(default)]
    external_id: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

async fn write_message(
    stdin: &mut ChildStdin,
    value: &serde_json::Value,
) -> Result<(), SourceError> {
    let mut text = serde_json::to_string(value)
        .map_err(|_| SourceError::permanent("MCP_ENCODE_FAILED", "could not encode request"))?;
    text.push('\n');
    stdin
        .write_all(text.as_bytes())
        .await
        .map_err(|_| SourceError::transient("MCP_PIPE_FAILED", "sidecar stdin closed"))?;
    stdin
        .flush()
        .await
        .map_err(|_| SourceError::transient("MCP_PIPE_FAILED", "sidecar stdin closed"))?;
    Ok(())
}

async fn read_message(
    reader: &mut BufReader<tokio::process::ChildStdout>,
    timeout: Duration,
) -> Result<serde_json::Value, SourceError> {
    let mut line = String::new();
    let read = tokio::time::timeout(timeout, reader.read_line(&mut line))
        .await
        .map_err(|_| SourceError::transient("MCP_TIMEOUT", "sidecar timed out"))?
        .map_err(|_| SourceError::transient("MCP_PIPE_FAILED", "could not read sidecar output"))?;

    if read == 0 {
        return Err(SourceError::transient(
            "MCP_PIPE_FAILED",
            "sidecar exited before responding",
        ));
    }

    serde_json::from_str(line.trim()).map_err(|_| {
        SourceError::transient("MCP_INVALID_RESPONSE", "sidecar returned invalid JSON-RPC")
    })
}

fn jsonrpc_error_message(error: &serde_json::Value) -> String {
    error
        .get("message")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("sidecar returned an error")
        .chars()
        .take(200)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(yaml: &str) -> SourceConfig {
        serde_yaml::from_str(yaml).expect("source config")
    }

    #[test]
    fn relative_cwd_is_resolved_against_the_sources_directory() {
        let source = source(
            r#"
id: jobspy
name: jobspy
source_type: mcp
config:
  command: uv
  args: ["run", "--project", "sidecar/jobspy_mcp", "jobspy-mcp-server"]
  cwd: "."
"#,
        );

        let adapter =
            McpAdapter::from_config(&source, Path::new("/home/dev/project")).expect("adapter");

        assert_eq!(adapter.cwd, Some(PathBuf::from("/home/dev/project")));
    }

    #[test]
    fn absolute_cwd_is_kept_verbatim() {
        let source = source(
            r#"
id: jobspy
name: jobspy
source_type: mcp
config:
  command: uv
  cwd: "/opt/jobspy"
"#,
        );

        let adapter =
            McpAdapter::from_config(&source, Path::new("/home/dev/project")).expect("adapter");

        assert_eq!(adapter.cwd, Some(PathBuf::from("/opt/jobspy")));
    }

    #[test]
    fn descriptions_are_fetched_by_default_and_can_be_disabled() {
        let default_source = source(
            r#"
id: jobspy
name: jobspy
source_type: mcp
config:
  command: uv
"#,
        );
        let adapter = McpAdapter::from_config(&default_source, Path::new("/home/dev/project"))
            .expect("adapter");
        assert!(adapter.fetch_descriptions);

        let disabled_source = source(
            r#"
id: jobspy
name: jobspy
source_type: mcp
config:
  command: uv
  fetch_descriptions: false
"#,
        );
        let adapter = McpAdapter::from_config(&disabled_source, Path::new("/home/dev/project"))
            .expect("adapter");
        assert!(!adapter.fetch_descriptions);
    }

    #[test]
    fn unset_cwd_inherits_the_application_directory() {
        let source = source(
            r#"
id: jobspy
name: jobspy
source_type: mcp
config:
  command: uv
"#,
        );

        let adapter =
            McpAdapter::from_config(&source, Path::new("/home/dev/project")).expect("adapter");

        assert_eq!(adapter.cwd, None);
    }
}
