import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getJobs } from '../lib/ipc';
import type { JobSummary } from '../lib/types';
import { NewJobsView } from './NewJobsView';

vi.mock('../lib/ipc', async () => {
  const { createIpcMock } = await import('../test/ipc-mock');
  return createIpcMock();
});

function job(index: number): JobSummary {
  return {
    job_id: `j-${index}`,
    title: `Rust Engineer ${index}`,
    company: 'Acme',
    location: 'Remote - Brazil',
    work_modality: 'Remote',
    url: `https://example.com/${index}`,
    posted_at: null,
    evaluation_status: 'APPROVED',
    lifecycle_state: 'DISCOVERED',
    match_score: 70 + (index % 30),
    matching_items_count: 3,
    total_requirements_count: 4,
    summary: 'Systems role',
    needs_review: false,
    applied_at: null,
  };
}

describe('feed rendering budget', () => {
  beforeEach(() => {
    vi.mocked(getJobs).mockResolvedValue({
      items: Array.from({ length: 12 }, (_, index) => job(index)),
      total: 1000,
      limit: 12,
      offset: 0,
      tab: 'NEW_JOBS',
    });
  });

  it('renders the paginated page without blocking the UI thread', async () => {
    const started = performance.now();
    render(
      <NewJobsView refreshToken={0} metrics={null} onSearch={() => {}} onOpenJob={() => {}} />,
    );
    await screen.findByText('Rust Engineer 0');
    const elapsed = performance.now() - started;

    // Loose regression guard; the strict 100 ms target is measured manually on
    // the reference machine because CI timing is noisy.
    expect(elapsed).toBeLessThan(1500);
    expect(screen.getByText('Rust Engineer 11')).toBeInTheDocument();
  });
});
