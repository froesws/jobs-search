import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { JobCard } from './JobCard';
import type { JobSummary } from '../lib/types';

const job: JobSummary = {
  job_id: 'job-1',
  title: 'Senior Rust Engineer',
  company: 'Acme',
  location: 'Remote - Brazil',
  work_modality: 'Remote',
  url: 'https://example.com/jobs/1',
  posted_at: '2026-11-08T12:00:00Z',
  evaluation_status: 'APPROVED',
  lifecycle_state: 'DISCOVERED',
  match_score: 92,
  matching_items_count: 4,
  total_requirements_count: 5,
  summary: 'Own the systems platform.',
  needs_review: false,
};

describe('JobCard', () => {
  it('shows the role, score, and N/M ratio', () => {
    const onOpen = vi.fn();
    render(<JobCard job={job} onOpen={onOpen} />);

    expect(screen.getByText('Senior Rust Engineer')).toBeInTheDocument();
    expect(screen.getByText('Acme')).toBeInTheDocument();
    expect(screen.getByLabelText('Match score 92')).toBeInTheDocument();
    expect(screen.getByText('4/5')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Senior Rust Engineer'));
    expect(onOpen).toHaveBeenCalledWith('job-1');
  });

  it('flags jobs that need identity review', () => {
    render(<JobCard job={{ ...job, needs_review: true }} onOpen={() => {}} />);
    expect(screen.getByText('Needs review')).toBeInTheDocument();
  });
});
