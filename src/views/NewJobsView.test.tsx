import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getJobs } from '@/lib/ipc';
import { FeedbackProvider } from '@/components/FeedbackProvider';
import { NewJobsView } from '@/views/NewJobsView';
import { jobFixture, searchMetricsFixture } from '@/test/fixtures';

vi.mock('@/lib/ipc', async () => {
  const { createIpcMock } = await import('@/test/ipc-mock');
  return createIpcMock();
});

const job = jobFixture;

const metrics = searchMetricsFixture;

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
      <FeedbackProvider>
        <NewJobsView refreshToken={1} metrics={metrics} onSearch={() => {}} onOpenJob={() => {}} />
      </FeedbackProvider>,
    );

    expect(await screen.findByText('Senior Rust Engineer')).toBeInTheDocument();
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
      <FeedbackProvider>
        <NewJobsView refreshToken={1} metrics={null} onSearch={() => {}} onOpenJob={() => {}} />
      </FeedbackProvider>,
    );

    expect(await screen.findByText('No approved jobs yet')).toBeInTheDocument();
  });

  it('renders aggregate progress while searching and clears it afterwards', () => {
    vi.mocked(getJobs).mockResolvedValue({
      items: [],
      total: 0,
      limit: 12,
      offset: 0,
      tab: 'NEW_JOBS',
    });

    const { rerender } = render(
      <FeedbackProvider>
        <NewJobsView
          refreshToken={1}
          metrics={null}
          onSearch={() => {}}
          onOpenJob={() => {}}
          searching
          progress={{ stage: 'collecting', found: 42, evaluated: 0, total: 0, failed: 1 }}
        />
      </FeedbackProvider>,
    );

    expect(screen.getByTestId('search-progress')).toHaveTextContent(
      'Collecting… 42 found · 1 source failed',
    );

    rerender(
      <FeedbackProvider>
        <NewJobsView
          refreshToken={1}
          metrics={null}
          onSearch={() => {}}
          onOpenJob={() => {}}
          searching
          progress={{ stage: 'evaluating', found: 42, evaluated: 37, total: 100, failed: 1 }}
        />
      </FeedbackProvider>,
    );
    expect(screen.getByTestId('search-progress')).toHaveTextContent('Evaluating… 37 of 100');

    rerender(
      <FeedbackProvider>
        <NewJobsView
          refreshToken={1}
          metrics={null}
          onSearch={() => {}}
          onOpenJob={() => {}}
          searching={false}
          progress={{ stage: 'evaluating', found: 42, evaluated: 100, total: 100, failed: 1 }}
        />
      </FeedbackProvider>,
    );
    expect(screen.queryByTestId('search-progress')).not.toBeInTheDocument();
  });
});
