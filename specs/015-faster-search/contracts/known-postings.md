# Contract: Known-posting lookup

**Feature**: `015-faster-search` | **Date**: 2026-11-12

The backend loads a read-only set of postings before a run and passes it to the LinkedIn adapter.
The adapter uses it to skip detail requests; the ingestion layer uses it to touch lineage.

---

## Shape

```rust
pub struct KnownPosting {
    pub external_id: String, // e.g. "linkedin-123"
    pub job_id: String,      // job_postings.id
    pub url: String,         // persisted URL
}
```

`IngestionRequest` carries `known_postings: Vec<KnownPosting>`.

---

## Query

```sql
SELECT l.external_id, l.job_id, l.url
FROM job_posting_sources l
JOIN job_postings j ON j.id = l.job_id
WHERE l.external_id LIKE 'linkedin-%'
  AND TRIM(j.description_clean) <> ''
GROUP BY l.external_id
```

**Rules**

- Only `linkedin-` identifiers are loaded. The prefix is produced by the LinkedIn MCP adapter,
  the scraper sidecar, and JobSpy's LinkedIn site.
- The lookup is global across sources; it is not restricted to `source_id = 'linkedin'`.
- A posting without a clean description is not in the set.
- A posting with several lineage rows contributes one entry per `external_id`.
- The query runs before ingestion, on the command thread, and never holds the database lock
  across an `.await`.

---

## Skip rules

| Situation | Detail request | Lineage | Counts as delivered |
|-----------|----------------|---------|---------------------|
| Listed ID is in the set | no | `ingested_at` updated for the LinkedIn source | yes |
| Listed ID is not in the set | yes | added by the normal ingestion path | yes if detail succeeds |
| Listed ID is in the set but the stored description is empty | yes | added/updated by the normal path | yes if detail succeeds |
| Detail fails for a new posting | yes (failed) | not added | no |
| Search fails before listing | not attempted | no | no |

**Rules**

- The adapter normalizes a raw search ID `123` to `linkedin-123` before the lookup. If the search
  returns a URL, the adapter extracts the same ID format.
- A skipped posting does not rewrite `job_postings`; `description_clean` stays as stored.
- `job_posting_sources.ingested_at` is updated with a single UPSERT, so a repeated skip does not
  create duplicate lineage rows.
- A skipped posting is not sent to the LLM unless its existing evaluation is stale.

---

## Frontend contract

None. The known set is internal to the backend and never exposed to the UI.
