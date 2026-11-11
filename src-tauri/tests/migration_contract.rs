use jobs_search_lib::db::Db;

#[test]
fn schema_has_no_raw_description_and_survives_reopen() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("jobs-search.sqlite3");
    let db = Db::open(&path).expect("open");

    let tables = table_names(&db);
    for expected in [
        "resumes",
        "candidate_profiles",
        "job_sources",
        "job_postings",
        "job_posting_sources",
        "job_evaluations",
        "job_applications",
        "job_status_events",
        "execution_runs",
        "run_metrics",
        "llm_metrics",
        "app_settings",
    ] {
        assert!(tables.iter().any(|table| table == expected), "{expected}");
    }

    for table in &tables {
        let columns = columns(&db, table);
        assert!(
            !columns
                .iter()
                .any(|column| column.contains("description_raw")),
            "table {table} must not store raw descriptions"
        );
    }

    let version: i64 = db
        .with_connection(|connection| {
            Ok(connection.pragma_query_value(None, "user_version", |row| row.get(0))?)
        })
        .expect("version");
    assert_eq!(version, 7);

    let resume_columns = columns(&db, "resumes");
    assert!(
        resume_columns
            .iter()
            .any(|column| column == "extracted_json"),
        "resumes must store the validated extraction for recalculation"
    );

    let approval_columns = columns(&db, "job_evaluations");
    assert!(
        approval_columns
            .iter()
            .any(|column| column == "approval_source"),
        "job_evaluations must record manual approval"
    );

    let evaluation_columns = columns(&db, "job_evaluations");
    assert!(
        evaluation_columns.iter().any(|column| column == "run_id"),
        "job_evaluations must link to the run that produced the evaluation"
    );

    let posting_columns = columns(&db, "job_postings");
    assert!(
        posting_columns
            .iter()
            .any(|column| column == "needs_review"),
        "job_postings must flag identity fallback for review"
    );

    drop(db);
    let reopened = Db::open(&path).expect("reopen");
    assert_eq!(table_names(&reopened).len(), tables.len());
    assert_eq!(
        reopened
            .with_connection(|connection| {
                Ok(
                    connection.query_row("SELECT COUNT(*) FROM job_postings", [], |row| {
                        row.get::<_, i64>(0)
                    })?,
                )
            })
            .expect("count"),
        0
    );
}

fn table_names(db: &Db) -> Vec<String> {
    db.with_connection(|connection| {
        let mut statement = connection
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        let mut names = Vec::new();
        for row in rows {
            names.push(row?);
        }
        Ok(names)
    })
    .expect("tables")
}

fn columns(db: &Db, table: &str) -> Vec<String> {
    db.with_connection(|connection| {
        let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
        let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
        let mut names = Vec::new();
        for row in rows {
            names.push(row?);
        }
        Ok(names)
    })
    .expect("columns")
}
