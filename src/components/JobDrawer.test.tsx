import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import type { JobDetail } from '../lib/types';
import { JobDrawer } from './JobDrawer';

const job: JobDetail = {
  job_id: 'j1',
  title: 'Senior Rust Engineer',
  company: 'Acme',
  location: 'Remote - Brazil',
  work_modality: 'Remote',
  url: 'https://example.com/jobs/1',
  posted_at: '2026-11-08T12:00:00Z',
  evaluation_status: 'APPROVED',
  lifecycle_state: 'DISCOVERED',
  match_score: 92,
  matching_items_count: 1,
  total_requirements_count: 2,
  summary: 'Systems role',
  needs_review: false,
  applied_at: null,
  description_clean: 'Build the systems platform.',
  requirements: [
    {
      id: 'r1',
      text: 'Rust',
      dimension: 'CORE_TECHNICAL',
      match_type: 'EXACT',
      evidence: 'projects',
    },
    {
      id: 'r2',
      text: 'gRPC',
      dimension: 'CORE_TECHNICAL',
      match_type: 'NONE',
      evidence: '',
    },
  ],
  matched_keywords: ['Rust'],
  missing_keywords: ['gRPC'],
  hard_filter_rejections: [],
  source_lineage: [
    {
      source_id: 'src',
      source_name: 'jobspy',
      external_id: '1',
      url: 'https://example.com/jobs/1',
      ingested_at: '2026-11-09T00:00:00Z',
    },
  ],
  score_breakdown: null,
  provider: 'deepseek',
  model: 'deepseek-chat',
  error_category: null,
};

interface DrawerCallbacks {
  onApply?: (jobId: string) => void;
  onIgnore?: (jobId: string) => void;
  onRestore?: (jobId: string) => void;
  onApprove?: (jobId: string) => void;
}

function renderDrawer(overrides: Partial<JobDetail> = {}, callbacks: DrawerCallbacks = {}) {
  return render(
    <JobDrawer
      open
      job={{ ...job, ...overrides }}
      status="ready"
      error={null}
      onClose={() => {}}
      onRetry={() => {}}
      {...callbacks}
    />,
  );
}

describe('JobDrawer', () => {
  it('shows the description, requirements, and matched/missing skills', () => {
    renderDrawer();
    expect(screen.getByText('Senior Rust Engineer')).toBeInTheDocument();
    expect(screen.getByText('Build the systems platform.')).toBeInTheDocument();
    expect(screen.getAllByText('Rust').length).toBeGreaterThan(0);
    expect(screen.getAllByText('gRPC').length).toBeGreaterThan(0);
    expect(screen.getByLabelText('Match score 92')).toBeInTheDocument();
  });

  it('shows lifecycle actions and invokes them', () => {
    const onApply = vi.fn();
    const onIgnore = vi.fn();
    renderDrawer({}, { onApply, onIgnore });

    fireEvent.click(screen.getByRole('button', { name: 'I Applied' }));
    fireEvent.click(screen.getByRole('button', { name: 'Ignore' }));
    expect(onApply).toHaveBeenCalledWith('j1');
    expect(onIgnore).toHaveBeenCalledWith('j1');
  });

  it('offers manual approval for a scored PENDING job', () => {
    const onApprove = vi.fn();
    renderDrawer({ evaluation_status: 'PENDING', match_score: 58 }, { onApprove });
    fireEvent.click(screen.getByRole('button', { name: /Approve manually/ }));
    expect(onApprove).toHaveBeenCalledWith('j1');
  });

  it('shows the no-requirements state and warns on invalid URLs', () => {
    renderDrawer({
      total_requirements_count: 0,
      matching_items_count: 0,
      requirements: [],
      url: 'javascript:alert(1)',
    });
    expect(screen.getByText('No requirements')).toBeInTheDocument();
    expect(screen.getByText(/original URL is missing or invalid/)).toBeInTheDocument();
  });
});
