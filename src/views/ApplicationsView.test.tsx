import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getJobs } from '../lib/ipc';
import type { JobSummary } from '../lib/types';
import { ApplicationsView } from './ApplicationsView';

vi.mock('../lib/ipc', () => ({
  getJobs: vi.fn(),
}));

const applied: JobSummary = {
  job_id: 'j1',
  title: 'Rust Engineer',
  company: 'Acme',
  location: 'Remote - Brazil',
  work_modality: 'Remote',
  url: 'https://example.com/1',
  posted_at: '2026-11-01T00:00:00Z',
  evaluation_status: 'APPROVED',
  lifecycle_state: 'APPLIED',
  match_score: 88,
  matching_items_count: 3,
  total_requirements_count: 4,
  summary: 'Systems',
  needs_review: false,
  applied_at: '2026-11-10T12:00:00Z',
};

describe('ApplicationsView', () => {
  beforeEach(() => {
    vi.mocked(getJobs).mockResolvedValue({
      items: [applied],
      total: 1,
      limit: 12,
      offset: 0,
      tab: 'APPLICATIONS',
    });
  });

  it('shows applied jobs with date, score, and requirement ratio', async () => {
    render(<ApplicationsView refreshToken={0} onOpenJob={() => {}} />);

    expect(await screen.findByText('Rust Engineer')).toBeInTheDocument();
    expect(screen.getByText('Acme')).toBeInTheDocument();
    expect(screen.getAllByText('3/4').length).toBeGreaterThan(0);
    expect(screen.getByText(/Nov/)).toBeInTheDocument();
    await waitFor(() => expect(getJobs).toHaveBeenCalledWith('APPLICATIONS', 12, 0));
  });
});
