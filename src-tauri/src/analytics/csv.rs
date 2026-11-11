//! CSV export for the current analytics view.

use std::io::{BufWriter, Write};
use std::path::Path;

use crate::analytics::{
    AnalyticsRequest, AnalyticsScope, ExportRequest, ExportResult, get_analytics,
};
use crate::db::Db;
use crate::error::AppError;

pub fn export(db: &Db, request: ExportRequest) -> Result<ExportResult, AppError> {
    let destination = Path::new(&request.destination);
    if destination.extension().and_then(|value| value.to_str()) != Some("csv") {
        return Err(AppError::validation(
            "the export destination must be a .csv file",
        ));
    }
    if destination.is_dir() {
        return Err(AppError::validation(
            "the export destination is a directory",
        ));
    }
    if let Some(parent) = destination.parent()
        && !parent.as_os_str().is_empty()
        && !parent.is_dir()
    {
        return Err(AppError::validation("the export directory does not exist"));
    }

    let analytics = get_analytics(
        db,
        AnalyticsRequest {
            scope: request.scope,
            run_id: request.run_id.clone(),
        },
    )?;
    let jobs = export_jobs(db, request.scope, analytics.run_id.as_deref())?;

    let file = std::fs::File::create(destination)?;
    let mut writer = BufWriter::new(file);
    write_pair(&mut writer, "metric", "value")?;
    write_pair(&mut writer, "scope", scope_label(request.scope))?;
    write_pair(
        &mut writer,
        "run_id",
        analytics.run_id.as_deref().unwrap_or(""),
    )?;
    write_pair(
        &mut writer,
        "denominator",
        &analytics.denominator.to_string(),
    )?;
    write_pair(&mut writer, "last_updated", &analytics.last_updated)?;
    write_pair(&mut writer, "stale", &analytics.stale.to_string())?;
    write_pair(
        &mut writer,
        "ingested",
        &analytics.funnel.ingested.to_string(),
    )?;
    write_pair(
        &mut writer,
        "duplicates",
        &analytics.funnel.duplicates.to_string(),
    )?;
    write_pair(
        &mut writer,
        "pending",
        &analytics.funnel.pending.to_string(),
    )?;
    write_pair(
        &mut writer,
        "approved",
        &analytics.funnel.approved.to_string(),
    )?;
    write_pair(&mut writer, "error", &analytics.funnel.error.to_string())?;
    write_pair(
        &mut writer,
        "applied",
        &analytics.funnel.applied.to_string(),
    )?;
    write_pair(
        &mut writer,
        "ignored",
        &analytics.funnel.ignored.to_string(),
    )?;
    write_pair(
        &mut writer,
        "approval_rate",
        &analytics.funnel.approval_rate.to_string(),
    )?;
    write_pair(
        &mut writer,
        "application_rate",
        &analytics.funnel.application_rate.to_string(),
    )?;
    for (reason, count) in &analytics.funnel.rejected_by_reason {
        write_pair(
            &mut writer,
            &format!("rejected.{reason}"),
            &count.to_string(),
        )?;
    }
    for (source, count) in &analytics.funnel.source_breakdown {
        write_pair(&mut writer, &format!("source.{source}"), &count.to_string())?;
    }
    for (provider, count) in &analytics.funnel.provider_breakdown {
        write_pair(
            &mut writer,
            &format!("provider.{provider}"),
            &count.to_string(),
        )?;
    }
    for bucket in &analytics.score_distribution {
        write_pair(
            &mut writer,
            &format!("score.{}", bucket.range),
            &bucket.count.to_string(),
        )?;
    }
    for metric in &analytics.skill_gap {
        write_pair(
            &mut writer,
            &format!("skill.{}", metric.display_name),
            &format!("{} ({:.1}%)", metric.demand_count, metric.demand_percentage),
        )?;
    }
    write_pair(
        &mut writer,
        "llm.calls",
        &analytics.llm_metrics.call_count.to_string(),
    )?;
    write_pair(
        &mut writer,
        "llm.p50_latency_ms",
        &analytics.llm_metrics.p50_latency_ms.to_string(),
    )?;
    write_pair(
        &mut writer,
        "llm.p95_latency_ms",
        &analytics.llm_metrics.p95_latency_ms.to_string(),
    )?;

    write_row(
        &mut writer,
        &[
            "title".to_string(),
            "company".to_string(),
            "location".to_string(),
            "work_modality".to_string(),
            "url".to_string(),
            "sources".to_string(),
            "evaluation_status".to_string(),
            "lifecycle_state".to_string(),
            "match_score".to_string(),
            "matching_items_count".to_string(),
            "total_requirements_count".to_string(),
            "matched_keywords".to_string(),
            "missing_keywords".to_string(),
            "description_clean".to_string(),
        ],
    )?;
    for job in &jobs {
        write_row(
            &mut writer,
            &[
                job.title.clone(),
                job.company.clone(),
                job.location.clone(),
                job.modality.clone(),
                job.url.clone(),
                job.sources.clone(),
                job.evaluation_status.clone(),
                job.lifecycle_state.clone(),
                job.score.map(|value| value.to_string()).unwrap_or_default(),
                job.matching
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                job.total.map(|value| value.to_string()).unwrap_or_default(),
                job.matched.clone(),
                job.missing.clone(),
                job.description.clone(),
            ],
        )?;
    }

    writer.flush()?;
    let bytes = std::fs::metadata(destination)?.len();
    let rows = u32::try_from(jobs.len())
        .map_err(|_| AppError::validation("too many rows for a single CSV export"))?;
    Ok(ExportResult {
        path: destination.display().to_string(),
        rows,
        bytes,
    })
}

struct ExportJob {
    title: String,
    company: String,
    location: String,
    modality: String,
    url: String,
    sources: String,
    evaluation_status: String,
    lifecycle_state: String,
    score: Option<i64>,
    matching: Option<i64>,
    total: Option<i64>,
    matched: String,
    missing: String,
    description: String,
}

fn export_jobs(
    db: &Db,
    scope: AnalyticsScope,
    run_id: Option<&str>,
) -> Result<Vec<ExportJob>, AppError> {
    db.with_connection(|connection| {
        let (sql, params): (&str, Vec<&str>) = match scope {
            AnalyticsScope::CurrentRun => (
                "SELECT j.title, j.company, j.location, j.work_modality, j.url,
                        COALESCE(GROUP_CONCAT(DISTINCT s.name), ''),
                        e.evaluation_status, COALESCE(a.lifecycle_state, 'DISCOVERED'),
                        e.match_score, e.matching_items_count, e.total_requirements_count,
                        COALESCE(e.matched_keywords_json, '[]'),
                        COALESCE(e.missing_keywords_json, '[]'),
                        j.description_clean
                 FROM job_evaluations e
                 JOIN job_postings j ON j.id = e.job_id
                 LEFT JOIN job_posting_sources l ON l.job_id = j.id
                 LEFT JOIN job_sources s ON s.id = l.source_id
                 LEFT JOIN job_applications a ON a.job_id = j.id
                 WHERE e.run_id = ?1 AND e.evaluation_status IN ('APPROVED', 'PENDING')
                 GROUP BY j.id ORDER BY e.match_score DESC, j.title ASC",
                vec![run_id.unwrap_or_default()],
            ),
            AnalyticsScope::Last7Days => (
                "SELECT j.title, j.company, j.location, j.work_modality, j.url,
                        COALESCE(GROUP_CONCAT(DISTINCT s.name), ''),
                        e.evaluation_status, COALESCE(a.lifecycle_state, 'DISCOVERED'),
                        e.match_score, e.matching_items_count, e.total_requirements_count,
                        COALESCE(e.matched_keywords_json, '[]'),
                        COALESCE(e.missing_keywords_json, '[]'),
                        j.description_clean
                 FROM job_evaluations e
                 JOIN job_postings j ON j.id = e.job_id
                 LEFT JOIN job_posting_sources l ON l.job_id = j.id
                 LEFT JOIN job_sources s ON s.id = l.source_id
                 LEFT JOIN job_applications a ON a.job_id = j.id
                 WHERE e.evaluation_status IN ('APPROVED', 'PENDING')
                   AND datetime(e.evaluated_at) >= datetime('now', '-7 days')
                 GROUP BY j.id ORDER BY e.match_score DESC, j.title ASC",
                Vec::new(),
            ),
        };
        let mut statement = connection.prepare(sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(params), |row| {
            Ok(ExportJob {
                title: row.get(0)?,
                company: row.get(1)?,
                location: row.get(2)?,
                modality: row.get(3)?,
                url: row.get(4)?,
                sources: row.get(5)?,
                evaluation_status: row.get(6)?,
                lifecycle_state: row.get(7)?,
                score: row.get(8)?,
                matching: row.get(9)?,
                total: row.get(10)?,
                matched: row.get(11)?,
                missing: row.get(12)?,
                description: row.get(13)?,
            })
        })?;
        let mut values = Vec::new();
        for row in rows {
            values.push(row?);
        }
        Ok(values)
    })
}

fn scope_label(scope: AnalyticsScope) -> &'static str {
    match scope {
        AnalyticsScope::CurrentRun => "CURRENT_RUN",
        AnalyticsScope::Last7Days => "LAST_7_DAYS",
    }
}

fn write_pair<W: Write>(writer: &mut W, key: &str, value: &str) -> Result<(), AppError> {
    write_row(writer, &[key.to_string(), value.to_string()])?;
    Ok(())
}

fn write_row<W: Write>(writer: &mut W, values: &[String]) -> Result<(), AppError> {
    let line = values
        .iter()
        .map(|value| escape(value))
        .collect::<Vec<_>>()
        .join(",");
    writeln!(writer, "{line}")?;
    Ok(())
}

fn escape(value: &str) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
