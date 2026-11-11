import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { JobDetail } from '@/lib/types';
import { JobDrawer } from '@/components/JobDrawer';
import { jobDetailFixture } from '@/test/fixtures';

const { openExternalUrl, notify } = vi.hoisted(() => ({
  openExternalUrl: vi.fn(),
  notify: vi.fn(),
}));

vi.mock('@/lib/ipc', () => ({ openExternalUrl }));
vi.mock('@/components/FeedbackProvider', () => ({
  useFeedback: () => ({ notify }),
}));

const job: JobDetail = jobDetailFixture;

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
  beforeEach(() => {
    openExternalUrl.mockReset();
    openExternalUrl.mockResolvedValue(undefined);
    notify.mockReset();
  });

  it('shows the description, requirements, and matched/missing skills', () => {
    renderDrawer();
    expect(screen.getByText('Senior Rust Engineer')).toBeInTheDocument();
    expect(screen.getByText('Build reliable systems.')).toBeInTheDocument();
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
    expect(onApply).toHaveBeenCalledWith(job.job_id);
    expect(onIgnore).toHaveBeenCalledWith(job.job_id);
  });

  it('offers manual approval for a scored PENDING job', () => {
    const onApprove = vi.fn();
    renderDrawer({ evaluation_status: 'PENDING', match_score: 58 }, { onApprove });
    fireEvent.click(screen.getByRole('button', { name: /Approve manually/ }));
    expect(onApprove).toHaveBeenCalledWith(job.job_id);
  });

  it('opens the original posting URL through the IPC wrapper when clicking the button', () => {
    renderDrawer();
    const button = screen.getByRole('button', { name: 'Open original posting' });

    expect(button).toHaveAttribute('data-testid', 'original-posting-url');
    expect(button).not.toHaveAttribute('href');
    expect(button).not.toHaveAttribute('target');
    expect(screen.getByTestId('original-posting-url-text')).toHaveTextContent(job.url);
    expect(screen.getByTestId('original-posting-url-text')).toHaveStyle({
      wordBreak: 'break-all',
      userSelect: 'text',
    });
    expect(screen.queryByRole('link', { name: job.url })).not.toBeInTheDocument();

    fireEvent.click(button);

    expect(openExternalUrl).toHaveBeenCalledTimes(1);
    expect(openExternalUrl).toHaveBeenCalledWith(job.url);
  });

  it('reports an open failure and keeps the drawer usable', async () => {
    openExternalUrl.mockRejectedValue(new Error('no browser handler'));
    renderDrawer();
    const button = screen.getByRole('button', { name: 'Open original posting' });

    fireEvent.click(button);

    await waitFor(() => expect(notify).toHaveBeenCalledTimes(1));
    expect(notify).toHaveBeenCalledWith(expect.stringContaining(job.url), 'error');
    expect(screen.getByTestId('original-posting-url-text')).toHaveTextContent(job.url);
    expect(button).toBeInTheDocument();
    expect(screen.getByText('Senior Rust Engineer')).toBeInTheDocument();
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

  it.each(['javascript:alert(1)', 'file:///etc/passwd', 'data:text/html,x', 'not a url'])(
    'warns and renders no posting link for %s',
    (url) => {
      renderDrawer({ url });

      expect(screen.getByText(/original URL is missing or invalid/)).toBeInTheDocument();
      expect(screen.queryByTestId('original-posting-url')).not.toBeInTheDocument();
      expect(openExternalUrl).not.toHaveBeenCalled();
    },
  );

  it('shows hard-filter reasons for a rejected evaluation', () => {
    renderDrawer({
      evaluation_status: 'REJECTED',
      match_score: null,
      hard_filter_rejections: [
        { code: 'SENIORITY_NOT_ELIGIBLE', message: 'seniority Executive is outside the range' },
      ],
    });
    expect(screen.getByText(/seniority Executive/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'I Applied' })).not.toBeInTheDocument();
  });
});
