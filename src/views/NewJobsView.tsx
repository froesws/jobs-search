import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Typography from '@mui/material/Typography';
import { useState } from 'react';

import type { JobSummary, SearchMetrics } from '../lib/types';
import { useJobsViewModel } from '../viewmodels/useJobsViewModel';
import { EmptyState } from '../components/EmptyState';
import { ErrorState } from '../components/ErrorState';
import { JobCard } from '../components/JobCard';
import { LoadingState } from '../components/LoadingState';

export function NewJobsView({
  refreshToken,
  metrics,
  onSearch,
  onOpenJob,
}: {
  refreshToken: number;
  metrics: SearchMetrics | null;
  onSearch: () => void;
  onOpenJob: (job: JobSummary) => void;
}) {
  const [tab, setTab] = useState<'NEW_JOBS' | 'BELOW_THRESHOLD'>('NEW_JOBS');
  const jobs = useJobsViewModel(tab, refreshToken);

  return (
    <Box sx={{ display: 'grid', gap: 2 }}>
      <Box sx={{ display: 'flex', alignItems: 'flex-end', gap: 2, flexWrap: 'wrap' }}>
        <Box sx={{ flex: 1 }}>
          <Typography variant="h1">
            {tab === 'NEW_JOBS' ? 'New Jobs' : 'Below threshold'}
          </Typography>
          <Typography color="text.secondary" variant="body2">
            {tab === 'NEW_JOBS'
              ? 'Approved matches ranked by score, requirement coverage, and recency.'
              : 'Pending evaluations that did not reach the approval threshold.'}
          </Typography>
        </Box>
        <Chip
          label={`${jobs.total} ${jobs.total === 1 ? 'job' : 'jobs'}`}
          variant="outlined"
          color="primary"
        />
        <ToggleButtonGroup
          exclusive
          size="small"
          value={tab}
          onChange={(_, value: 'NEW_JOBS' | 'BELOW_THRESHOLD' | null) => {
            if (value) setTab(value);
          }}
          aria-label="Job visibility filter"
        >
          <ToggleButton value="NEW_JOBS">Approved</ToggleButton>
          <ToggleButton value="BELOW_THRESHOLD">Below threshold</ToggleButton>
        </ToggleButtonGroup>
      </Box>

      {metrics ? <RunSummary metrics={metrics} /> : null}

      {jobs.status === 'loading' ? <LoadingState label="Fetching your ranked matches…" /> : null}
      {jobs.status === 'error' ? (
        <ErrorState message={jobs.error ?? 'Unknown backend error'} onRetry={jobs.retry} />
      ) : null}
      {jobs.status === 'empty' ? (
        <EmptyState
          title={tab === 'NEW_JOBS' ? 'No approved jobs yet' : 'No jobs below the threshold'}
          description={
            tab === 'NEW_JOBS'
              ? 'Run a search after configuring your filters. Approved matches will appear here, ranked and explainable.'
              : 'Every evaluated job either passed the threshold or was rejected by a hard filter.'
          }
          action={
            <Button variant="contained" onClick={onSearch}>
              Refresh Search
            </Button>
          }
        />
      ) : null}
      {jobs.status === 'ready' ? (
        <>
          <Box
            sx={{
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fill, minmax(330px, 1fr))',
              gap: 2,
            }}
          >
            {jobs.items.map((job) => (
              <JobCard key={job.job_id} job={job} onOpen={() => onOpenJob(job)} />
            ))}
          </Box>
          <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Typography variant="caption" color="text.secondary">
              Showing {jobs.offset + 1}–{Math.min(jobs.offset + jobs.limit, jobs.total)} of{' '}
              {jobs.total}
            </Typography>
            <Box sx={{ display: 'flex', gap: 1 }}>
              <Button
                size="small"
                variant="outlined"
                disabled={!jobs.hasPrevious}
                onClick={jobs.previousPage}
              >
                Previous
              </Button>
              <Button
                size="small"
                variant="outlined"
                disabled={!jobs.hasNext}
                onClick={jobs.nextPage}
              >
                Next
              </Button>
            </Box>
          </Box>
        </>
      ) : null}
    </Box>
  );
}

function RunSummary({ metrics }: { metrics: SearchMetrics }) {
  const hasFailures = metrics.source_failures.length > 0;
  const severity = metrics.status === 'FAILED' ? 'error' : hasFailures ? 'warning' : 'success';
  return (
    <Alert severity={severity} variant="outlined" sx={{ alignItems: 'center' }}>
      <Typography variant="body2" sx={{ fontWeight: 700 }}>
        {metrics.status === 'FAILED' ? 'Search failed' : 'Search completed'} ·{' '}
        {metrics.approved_count} approved, {metrics.pending_count} pending, {metrics.rejected_count}{' '}
        rejected, {metrics.error_count} errors
      </Typography>
      <Typography variant="caption" color="text.secondary">
        {metrics.total_ingested} ingested · {metrics.duplicates_merged} duplicates merged · run{' '}
        {metrics.run_id.slice(0, 8)}
      </Typography>
      {hasFailures ? (
        <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 0.5 }}>
          Source failures: {metrics.source_failures.join(' · ')}
        </Typography>
      ) : null}
    </Alert>
  );
}
