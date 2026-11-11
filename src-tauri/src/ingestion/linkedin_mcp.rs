//! LinkedIn MCP source (stickerdaniel/linkedin-mcp-server).
#![allow(missing_docs)]

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout, Command};
use zeroize::Zeroizing;

use crate::config::SourceConfig;
use crate::error::AppError;
use crate::ingestion::SourcePacer;
use crate::ingestion::adapter::{IngestionRequest, RawJob, SourceAdapter, SourceError};
use crate::types::{Seniority, WorkModality};

const PROTOCOL_VERSION: &str = "2024-11-05";
const MAX_RESPONSE_LINES: usize = 200;

pub struct LinkedInMcpAdapter {
    id: String,
    name: String,
    command: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    results_wanted: u32,
    max_pages: u32,
    timeout: Duration,
    pacer: SourcePacer,
}

impl LinkedInMcpAdapter {
    pub fn from_config(source: &SourceConfig) -> Result<Self, AppError> {
        let command = source
            .config_str("command")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::Config(format!(
                    "linkedin mcp source {} requires config.command",
                    source.id
                ))
            })?
            .to_string();

        Ok(Self {
            id: source.id.clone(),
            name: source.name.clone(),
            command,
            args: source.config_string_list("args"),
            cwd: source
                .config_str("cwd")
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from),
            results_wanted: source.config_u32("results_wanted", 10).clamp(1, 25),
            max_pages: source.config_u32("max_pages", 1).clamp(1, 3),
            timeout: Duration::from_secs(source.rate_limit.timeout_seconds.clamp(1, 600)),
            pacer: SourcePacer::new(source.rate_limit.requests_per_minute),
        })
    }

    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
        results_wanted: u32,
        max_pages: u32,
        timeout: Duration,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            command: command.into(),
            args,
            cwd: None,
            results_wanted: results_wanted.clamp(1, 25),
            max_pages: max_pages.clamp(1, 3),
            timeout,
            pacer: SourcePacer::new(60_000),
        }
    }

    async fn exchange(
        &self,
        request: &IngestionRequest,
        stdin: ChildStdin,
        stdout: ChildStdout,
    ) -> Result<Vec<RawJob>, SourceError> {
        let mut client = McpClient {
            stdin,
            reader: BufReader::new(stdout),
            timeout: self.timeout,
            next_id: 1,
        };
        client.initialize().await?;

        let total_limit = if request.results_wanted == 0 {
            self.results_wanted
        } else {
            request.results_wanted.min(self.results_wanted)
        };
        let queries = if request.queries.is_empty() {
            vec![request.query.clone()]
        } else {
            request.queries.clone()
        };
        let per_query = (total_limit / queries.len().max(1) as u32).max(1);

        let mut jobs = Vec::new();
        let mut detail_failures = 0_u32;
        for query in queries {
            let mut arguments = json!({
                "keywords": query,
                "location": request.location,
                "max_pages": self.max_pages,
            });
            if let Some(date_posted) = date_filter(request.hours_old) {
                arguments["date_posted"] = json!(date_posted);
            }
            if let Some(experience) = experience_filter(&request.seniorities) {
                arguments["experience_level"] = json!(experience);
            }
            if let Some(work_type) = work_type_filter(&request.modalities) {
                arguments["work_type"] = json!(work_type);
            }

            let search = client.call_tool("search_jobs", &arguments).await?;
            let payload = tool_payload(&search)?;
            let ids = payload
                .get("job_ids")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();

            for id in ids.iter().take(per_query as usize) {
                let Some(id) = job_id_value(id) else {
                    continue;
                };
                let detail = match client
                    .call_tool("get_job_details", &json!({ "job_id": id }))
                    .await
                {
                    Ok(value) => value,
                    Err(error) => {
                        detail_failures += 1;
                        tracing::warn!(
                            provider = "linkedin",
                            code = error.code,
                            "linkedin job detail failed"
                        );
                        continue;
                    }
                };
                let payload = match tool_payload(&detail) {
                    Ok(value) => value,
                    Err(_) => {
                        detail_failures += 1;
                        continue;
                    }
                };
                let url = payload
                    .get("url")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("https://www.linkedin.com/jobs/view/{id}/"));
                let text = payload
                    .pointer("/sections/job_posting")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if let Some(job) = parse_job_posting(&url, text) {
                    jobs.push(job);
                } else {
                    detail_failures += 1;
                }
            }
        }

        if jobs.is_empty() && detail_failures > 0 {
            return Err(SourceError::transient(
                "LINKEDIN_DETAILS_FAILED",
                "LinkedIn did not return readable job details",
            ));
        }

        let mut seen = HashSet::new();
        jobs.retain(|job| seen.insert(job.external_id.clone()));
        Ok(jobs)
    }
}

#[async_trait]
impl SourceAdapter for LinkedInMcpAdapter {
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
            .kill_on_drop(true);
        if let Some(cwd) = &self.cwd {
            command.current_dir(cwd);
        }
        let mut child = command.spawn().map_err(|_| {
            SourceError::permanent(
                "LINKEDIN_START_FAILED",
                "could not start the LinkedIn MCP server",
            )
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            SourceError::permanent("LINKEDIN_PIPE_FAILED", "LinkedIn MCP stdin is unavailable")
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            SourceError::permanent("LINKEDIN_PIPE_FAILED", "LinkedIn MCP stdout is unavailable")
        })?;

        let result = self.exchange(request, stdin, stdout).await;
        let _ = child.kill().await;
        let _ = child.wait().await;
        result
    }
}

struct McpClient {
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
    timeout: Duration,
    next_id: u64,
}

impl McpClient {
    async fn initialize(&mut self) -> Result<(), SourceError> {
        let id = self.next_id;
        self.next_id += 1;
        self.write(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "jobs-search", "version": "0.1.0" }
            }
        }))
        .await?;
        let response = self.read_response(id).await?;
        if let Some(error) = response.get("error") {
            return Err(SourceError::transient(
                "LINKEDIN_INIT_FAILED",
                jsonrpc_error_message(error),
            ));
        }
        self.write(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))
        .await?;
        Ok(())
    }

    async fn call_tool(&mut self, name: &str, arguments: &Value) -> Result<Value, SourceError> {
        let id = self.next_id;
        self.next_id += 1;
        self.write(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        }))
        .await?;
        let response = self.read_response(id).await?;
        if let Some(error) = response.get("error") {
            return Err(SourceError::transient(
                "LINKEDIN_TOOL_ERROR",
                jsonrpc_error_message(error),
            ));
        }
        response.get("result").cloned().ok_or_else(|| {
            SourceError::transient(
                "LINKEDIN_INVALID_RESPONSE",
                "LinkedIn MCP response has no result",
            )
        })
    }

    async fn write(&mut self, value: &Value) -> Result<(), SourceError> {
        let mut text = serde_json::to_string(value).map_err(|_| {
            SourceError::permanent("LINKEDIN_ENCODE_FAILED", "could not encode MCP request")
        })?;
        text.push('\n');
        self.stdin.write_all(text.as_bytes()).await.map_err(|_| {
            SourceError::transient("LINKEDIN_PIPE_FAILED", "LinkedIn MCP stdin closed")
        })?;
        self.stdin.flush().await.map_err(|_| {
            SourceError::transient("LINKEDIN_PIPE_FAILED", "LinkedIn MCP stdin closed")
        })?;
        Ok(())
    }

    async fn read_response(&mut self, id: u64) -> Result<Value, SourceError> {
        for _ in 0..MAX_RESPONSE_LINES {
            let message = read_message(&mut self.reader, self.timeout).await?;
            if message
                .get("id")
                .and_then(Value::as_u64)
                .is_some_and(|value| value == id)
            {
                return Ok(message);
            }
        }
        Err(SourceError::transient(
            "LINKEDIN_INVALID_RESPONSE",
            "LinkedIn MCP did not return the requested response",
        ))
    }
}

async fn read_message(
    reader: &mut BufReader<ChildStdout>,
    timeout: Duration,
) -> Result<Value, SourceError> {
    let mut line = String::new();
    let read = tokio::time::timeout(timeout, reader.read_line(&mut line))
        .await
        .map_err(|_| SourceError::transient("LINKEDIN_TIMEOUT", "LinkedIn MCP timed out"))?
        .map_err(|_| {
            SourceError::transient("LINKEDIN_PIPE_FAILED", "could not read LinkedIn MCP output")
        })?;
    if read == 0 {
        return Err(SourceError::transient(
            "LINKEDIN_PIPE_FAILED",
            "LinkedIn MCP exited before responding",
        ));
    }
    serde_json::from_str(line.trim()).map_err(|_| {
        SourceError::transient(
            "LINKEDIN_INVALID_RESPONSE",
            "LinkedIn MCP returned invalid JSON-RPC",
        )
    })
}

fn tool_payload(result: &Value) -> Result<Value, SourceError> {
    if let Some(structured) = result.get("structuredContent")
        && !structured.is_null()
    {
        return Ok(structured.clone());
    }
    if let Some(content) = result.get("content").and_then(Value::as_array) {
        for entry in content {
            if let Some(text) = entry.get("text").and_then(Value::as_str)
                && let Ok(value) = serde_json::from_str(text)
            {
                return Ok(value);
            }
        }
    }
    if result.is_object() {
        return Ok(result.clone());
    }
    Err(SourceError::transient(
        "LINKEDIN_INVALID_RESPONSE",
        "LinkedIn MCP returned an unsupported payload",
    ))
}

fn jsonrpc_error_message(error: &Value) -> String {
    error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("LinkedIn MCP returned an error")
        .chars()
        .take(200)
        .collect()
}

fn job_id_value(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.as_i64().map(|number| number.to_string()))
}

fn date_filter(hours_old: u32) -> Option<&'static str> {
    match hours_old {
        0..=1 => Some("past_hour"),
        2..=24 => Some("past_24_hours"),
        25..=168 => Some("past_week"),
        169..=720 => Some("past_month"),
        _ => None,
    }
}

fn experience_filter(seniorities: &[Seniority]) -> Option<String> {
    if seniorities.is_empty() || seniorities.len() >= 5 {
        return None;
    }
    let mut values = Vec::new();
    for seniority in seniorities {
        let mapped: &[&str] = match seniority {
            Seniority::Trainee | Seniority::Intern => &["internship", "entry"],
            Seniority::Junior => &["entry", "associate"],
            Seniority::Pleno => &["associate", "mid_senior"],
            Seniority::Senior => &["mid_senior", "director", "executive"],
        };
        for value in mapped {
            if !values.contains(value) {
                values.push(*value);
            }
        }
    }
    (!values.is_empty()).then(|| values.join(","))
}

fn work_type_filter(modalities: &[WorkModality]) -> Option<String> {
    if modalities.is_empty() || modalities.len() >= 3 {
        return None;
    }
    let mut values = Vec::new();
    for modality in modalities {
        let value = match modality {
            WorkModality::Remote => "remote",
            WorkModality::Hybrid => "hybrid",
            WorkModality::Onsite => "on_site",
            _ => continue,
        };
        if !values.contains(&value) {
            values.push(value);
        }
    }
    (!values.is_empty()).then(|| values.join(","))
}

fn parse_job_posting(url: &str, text: &str) -> Option<RawJob> {
    let noise = [
        "Vaga verificada",
        "Promovida",
        "Salva",
        "Candidatura simplificada",
        "Avaliando candidaturas",
        "Seja uma das primeiras pessoas a se candidatar",
        "Você seria um dos melhores candidatos",
    ];
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !noise.contains(line))
        .collect();
    if lines.len() < 2 {
        return None;
    }

    let company = lines[0].to_string();
    let title = lines[1].to_string();
    if title.is_empty() {
        return None;
    }

    let header = lines
        .iter()
        .find(|line| line.contains(" · "))
        .copied()
        .unwrap_or_default();
    let location = header
        .split('·')
        .next()
        .unwrap_or("Unknown")
        .split(" (")
        .next()
        .unwrap_or("Unknown")
        .trim()
        .to_string();
    let location = if location.is_empty() {
        "Unknown".to_string()
    } else {
        location
    };

    let lower = text.to_ascii_lowercase();
    let modality = if lower.contains("híbrido")
        || lower.contains("hibrido")
        || lower.contains("hybrid")
    {
        Some("Hybrid".to_string())
    } else if lower.contains("remoto") || lower.contains("remote") {
        Some("Remote".to_string())
    } else if lower.contains("presencial") || lower.contains("on-site") || lower.contains("onsite")
    {
        Some("Onsite".to_string())
    } else {
        None
    };

    let description = extract_description(&lines);
    let description = if description.trim().is_empty() {
        lines.iter().skip(3).copied().collect::<Vec<_>>().join("\n")
    } else {
        description
    };

    let external_id =
        job_id_from_url(url).unwrap_or_else(|| crate::dedup::identity::sha256_hex(url));

    Some(RawJob {
        external_id,
        title,
        company,
        location,
        work_modality: modality,
        url: url.to_string(),
        posted_at: None,
        description: Zeroizing::new(description),
        source_name: "linkedin".to_string(),
        tags: Vec::new(),
    })
}

fn extract_description(lines: &[&str]) -> String {
    let heading = lines.iter().position(|line| {
        let lower = line.to_ascii_lowercase();
        lower.starts_with("sobre a vaga")
            || lower.starts_with("sobre a vagas")
            || lower.starts_with("about the job")
            || lower.starts_with("descrição da vaga")
    });
    let body: &[&str] = match heading {
        Some(index) => &lines[index + 1..],
        None => lines.get(3..).unwrap_or(&[]),
    };
    let stops = [
        "ative um alerta",
        "set alert",
        "mais vagas",
        "similar jobs",
        "exibir mais",
        "veja como sua candidatura",
        "pessoas que clicaram",
        "sobre a empresa",
        "company overview",
        "buscando talentos",
        "looking for talent",
    ];
    body.iter()
        .take_while(|line| {
            let lower = line.to_ascii_lowercase();
            !stops.iter().any(|stop| lower.starts_with(stop))
        })
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
}

fn job_id_from_url(url: &str) -> Option<String> {
    let rest = url.split("/jobs/view/").nth(1)?;
    let id = rest.split(['/', '?']).next()?;
    (!id.is_empty()).then(|| format!("linkedin-{id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_linkedin_job_posting() {
        let text = "Acme\n\
            Rust Engineer\n\
            Porto Alegre, Brasil  · Compartilhada há 2 dias · 10 pessoas clicaram\n\
            Presencial\n\
            Sobre a vaga\n\
            We need Rust and Linux.\n\
            Ative um alerta para vagas semelhantes\n\
            Mais vagas\n\
            Other job";
        let job = parse_job_posting("https://www.linkedin.com/jobs/view/123/", text).expect("job");
        assert_eq!(job.title, "Rust Engineer");
        assert_eq!(job.company, "Acme");
        assert_eq!(job.location, "Porto Alegre, Brasil");
        assert_eq!(job.work_modality.as_deref(), Some("Onsite"));
        assert_eq!(job.external_id, "linkedin-123");
        assert!(job.description.contains("Rust and Linux"));
        assert!(!job.description.contains("Other job"));
    }

    #[test]
    fn maps_date_experience_and_work_type_filters() {
        assert_eq!(date_filter(1), Some("past_hour"));
        assert_eq!(date_filter(24), Some("past_24_hours"));
        assert_eq!(date_filter(168), Some("past_week"));
        assert_eq!(date_filter(720), Some("past_month"));
        assert_eq!(date_filter(721), None);

        assert_eq!(
            experience_filter(&[Seniority::Trainee, Seniority::Junior]).as_deref(),
            Some("internship,entry,associate")
        );
        assert_eq!(experience_filter(&[]), None);
        assert_eq!(
            work_type_filter(&[WorkModality::Remote, WorkModality::Hybrid]).as_deref(),
            Some("remote,hybrid")
        );
        assert_eq!(
            work_type_filter(&[
                WorkModality::Remote,
                WorkModality::Hybrid,
                WorkModality::Onsite
            ]),
            None
        );
    }

    #[test]
    fn extracts_job_ids_from_links() {
        assert_eq!(
            job_id_from_url("https://www.linkedin.com/jobs/view/123/?x=1").as_deref(),
            Some("linkedin-123")
        );
        assert_eq!(job_id_from_url("https://example.com/jobs/view/"), None);
    }
}
