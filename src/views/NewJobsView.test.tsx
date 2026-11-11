import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getJobs } from '../lib/ipc';
import type { JobSummary, SearchMetrics } from '../lib/types';
import { NewJobsView } from './NewJobsView';

vi.mock('../lib/ipc', async () => {
  const { createIpcMock } = await import('../test/ipc-mock');
  return createIpcMock();
});

const job: JobSummary = {
  job_id: 'j1',
  title: 'Rust Engineer',
  company: 'Acme',
  location: 'Remote - Brazil',
  work_modality: 'Remote',
  url: 'https://example.com/1',
  posted_at: null,
  evaluation_status: 'APPROVED',
  lifecycle_state: 'DISCOVERED',
  match_score: 88,
  matching_items_count: 2,
  total_requirements_count: 3,
  summary: 'Systems role',
  needs_review: false,
  applied_at: null,
};

const metrics: SearchMetrics = {
  run_id: 'run-1',
  status: 'PARTIAL',
  total_ingested: 5,
  duplicates_merged: 1,
  rejected_count: 1,
  pending_count: 0,
  approved_count: 1,
  error_count: 0,
  started_at: '2026-11-10T10:00:00Z',
  finished_at: '2026-11-10T10:01:00Z',
  source_failures: ['jobspy: timeout'],
};

describe('NewJobsView', () => {
  beforeEach(() => {
    vi.mocked(getJobs).mockReset();
  });

  it('keeps successful cards visible when one source fails', async () => {
    vi.mocked(getJobs).mockResolvedValue({
      items: [job],
      total: 1,
      limit: 12,
      offset: 0,
      tab: 'NEW_JOBS',
    });

    render(
      <NewJobsView refreshToken={1} metrics={metrics} onSearch={() => {}} onOpenJob={() => {}} />,
    );

    expect(await screen.findByText('Rust Engineer')).toBeInTheDocument();
    expect(screen.getByText(/Source failures: jobspy: timeout/)).toBeInTheDocument();
    expect(screen.getByText(/Search completed/)).toBeInTheDocument();
  });

  it('shows a clear zero-result state', async () => {
    vi.mocked(getJobs).mockResolvedValue({
      items: [],
      total: 0,
      limit: 12,
      offset: 0,
      tab: 'NEW_JOBS',
    });

    render(
      <NewJobsView refreshToken={1} metrics={null} onSearch={() => {}} onOpenJob={() => {}} />,
    );

    expect(await screen.findByText('No approved jobs yet')).toBeInTheDocument();
  });
});
