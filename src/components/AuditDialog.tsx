import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Divider from '@mui/material/Divider';
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableContainer from '@mui/material/TableContainer';
import TableHead from '@mui/material/TableHead';
import TableRow from '@mui/material/TableRow';
import Typography from '@mui/material/Typography';
import { save } from '@tauri-apps/plugin-dialog';
import { useCallback, useEffect, useState } from 'react';

import { safeMessage } from '@/lib/errors';
import { exportRunJobsCsv, getAudit, getRunJobs } from '@/lib/ipc';
import type { ExecutionAudit, PaginatedRunJobs } from '@/lib/types';
import { ErrorState } from '@/components/ErrorState';
import { useFeedback } from '@/components/FeedbackProvider';
import { Metric } from '@/components/Metric';
import { LoadingState } from '@/components/LoadingState';

const PAGE_SIZE = 10;
const EMPTY_PAGE: PaginatedRunJobs = { items: [], total: 0, limit: PAGE_SIZE, offset: 0 };

export function AuditDialog({
  runId,
  open,
  onClose,
}: {
  runId: string | null;
  open: boolean;
  onClose: () => void;
}) {
  const feedback = useFeedback();
  const [audit, setAudit] = useState<ExecutionAudit | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [jobs, setJobs] = useState<PaginatedRunJobs>(EMPTY_PAGE);
  const [page, setPage] = useState(0);
  const [jobsError, setJobsError] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);

  useEffect(() => {
    if (!open || !runId) return;
    let active = true;
    setAudit(null);
    setError(null);
    setPage(0);
    getAudit(runId)
      .then((value) => {
        if (active) setAudit(value);
      })
      .catch((cause) => {
        if (active) setError(safeMessage(cause));
      });
    return () => {
      active = false;
    };
  }, [open, runId]);

  useEffect(() => {
    if (!open || !runId) return;
    let active = true;
    setJobsError(null);
    getRunJobs(runId, PAGE_SIZE, page * PAGE_SIZE)
      .then((value) => {
        if (active) setJobs(value);
      })
      .catch((cause) => {
        if (active) setJobsError(safeMessage(cause));
      });
    return () => {
      active = false;
    };
  }, [open, runId, page]);

  const handleExport = useCallback(async () => {
    if (!runId) return;
    const stamp = new Date().toISOString().slice(0, 10);
    try {
      const selected = await save({
        defaultPath: `jobs-search-run-${runId.slice(0, 8)}-${stamp}.csv`,
        filters: [{ name: 'CSV', extensions: ['csv'] }],
      });
      if (!selected) return;
      setExporting(true);
      const result = await exportRunJobsCsv(runId, selected);
      feedback.notify(
        `Exported ${result.rows} job${result.rows === 1 ? '' : 's'} to ${result.path}.`,
        'success',
      );
    } catch (cause) {
      feedback.notify(safeMessage(cause), 'error', true);
    } finally {
      setExporting(false);
    }
  }, [runId, feedback]);

  const pageCount = Math.max(1, Math.ceil(jobs.total / PAGE_SIZE));

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="lg">
      <DialogTitle>Execution audit</DialogTitle>
      <DialogContent dividers>
        {!audit && !error ? <LoadingState label="Loading the run audit…" /> : null}
        {error ? <ErrorState message={error} /> : null}
        {audit ? (
          <Box sx={{ display: 'grid', gap: 2 }}>
            <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
              <Metric label="Status" value={audit.status} />
              <Metric label="Ingested" value={String(audit.total_ingested)} />
              <Metric label="Duplicates" value={String(audit.duplicates_merged)} />
              <Metric label="Rejected" value={String(audit.rejected_count)} />
              <Metric label="Pending" value={String(audit.pending_count)} />
              <Metric label="Approved" value={String(audit.approved_count)} />
              <Metric label="Errors" value={String(audit.error_count)} />
              <Metric label="Applied" value={String(audit.applied_count)} />
              <Metric label="Ignored" value={String(audit.ignored_count)} />
              <Metric label="Approval rate" value={`${audit.approval_rate.toFixed(1)}%`} />
              <Metric label="Application rate" value={`${audit.application_rate.toFixed(1)}%`} />
            </Box>
            <Divider />
            <Section title="Rejections by reason">
              {Object.keys(audit.rejected_by_reason).length === 0 ? (
                <Muted>No hard-filter rejections.</Muted>
              ) : (
                Object.entries(audit.rejected_by_reason).map(([reason, count]) => (
                  <Row key={reason} label={reason} value={String(count)} />
                ))
              )}
            </Section>
            <Section title="Source failures">
              {audit.source_failures.length === 0 ? (
                <Muted>No source failures.</Muted>
              ) : (
                <Alert severity="warning" variant="outlined">
                  {audit.source_failures.join(' · ')}
                </Alert>
              )}
            </Section>
            <Box sx={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 2 }}>
              <Section title="By source">
                {Object.entries(audit.source_breakdown).map(([source, count]) => (
                  <Row key={source} label={source} value={String(count)} />
                ))}
              </Section>
              <Section title="By provider/model">
                {Object.entries(audit.provider_breakdown).map(([provider, count]) => (
                  <Row key={provider} label={provider} value={String(count)} />
                ))}
              </Section>
            </Box>
            <Section title="Score distribution">
              <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap' }}>
                {Object.entries(audit.score_distribution).map(([bucket, count]) => (
                  <Metric key={bucket} label={bucket} value={String(count)} />
                ))}
              </Box>
            </Section>
            <Divider />
            <Section title={`Jobs found (${jobs.total})`}>
              {jobsError ? <ErrorState message={jobsError} /> : null}
              {!jobsError && jobs.items.length === 0 ? (
                <Muted>No jobs were evaluated by this run.</Muted>
              ) : null}
              {jobs.items.length > 0 ? (
                <>
                  <TableContainer>
                    <Table size="small">
                      <TableHead>
                        <TableRow>
                          <TableCell>Title</TableCell>
                          <TableCell>Company</TableCell>
                          <TableCell>Location</TableCell>
                          <TableCell>Status</TableCell>
                          <TableCell align="right">Score</TableCell>
                          <TableCell>Reason</TableCell>
                        </TableRow>
                      </TableHead>
                      <TableBody>
                        {jobs.items.map((job) => (
                          <TableRow key={job.jobId}>
                            <TableCell>{job.title}</TableCell>
                            <TableCell>{job.company}</TableCell>
                            <TableCell>{job.location}</TableCell>
                            <TableCell>{job.evaluationStatus ?? '—'}</TableCell>
                            <TableCell align="right">{job.matchScore ?? '—'}</TableCell>
                            <TableCell>{job.rejectionReason ?? '—'}</TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  </TableContainer>
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mt: 1 }}>
                    <Button
                      size="small"
                      disabled={page === 0}
                      onClick={() => setPage((value) => Math.max(0, value - 1))}
                    >
                      Previous
                    </Button>
                    <Typography variant="caption" color="text.secondary">
                      Page {page + 1} of {pageCount}
                    </Typography>
                    <Button
                      size="small"
                      disabled={page + 1 >= pageCount}
                      onClick={() => setPage((value) => value + 1)}
                    >
                      Next
                    </Button>
                  </Box>
                </>
              ) : null}
            </Section>
          </Box>
        ) : null}
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Close</Button>
        <Button
          variant="contained"
          disabled={exporting || !runId}
          onClick={() => void handleExport()}
        >
          Export jobs CSV
        </Button>
      </DialogActions>
    </Dialog>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <Box sx={{ display: 'grid', gap: 0.75 }}>
      <Typography variant="h3">{title}</Typography>
      {children}
    </Box>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <Box sx={{ display: 'flex', justifyContent: 'space-between', gap: 2 }}>
      <Typography variant="body2" color="text.secondary">
        {label}
      </Typography>
      <Typography variant="body2" sx={{ fontWeight: 700 }}>
        {value}
      </Typography>
    </Box>
  );
}

function Muted({ children }: { children: React.ReactNode }) {
  return (
    <Typography variant="body2" color="text.secondary">
      {children}
    </Typography>
  );
}
