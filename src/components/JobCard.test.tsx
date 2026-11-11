import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { JobCard } from '@/components/JobCard';
import { jobFixture } from '@/test/fixtures';

const job = jobFixture;

describe('JobCard', () => {
  it('shows the role, score, and N/M ratio', () => {
    const onOpen = vi.fn();
    render(<JobCard job={job} onOpen={onOpen} />);

    expect(screen.getByText('Senior Rust Engineer')).toBeInTheDocument();
    expect(screen.getByText('Acme')).toBeInTheDocument();
    expect(screen.getByLabelText('Match score 92')).toBeInTheDocument();
    expect(screen.getByText('4/5')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Senior Rust Engineer'));
    expect(onOpen).toHaveBeenCalledWith(job.job_id);
  });

  it('flags jobs that need identity review', () => {
    render(<JobCard job={{ ...job, needs_review: true }} onOpen={() => {}} />);
    expect(screen.getByText('Needs review')).toBeInTheDocument();
  });
});
