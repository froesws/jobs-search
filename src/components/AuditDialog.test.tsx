import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getAudit, getRunJobs } from '@/lib/ipc';
import { auditFixture } from '@/test/fixtures';
import { AuditDialog } from '@/components/AuditDialog';
import { FeedbackProvider } from '@/components/FeedbackProvider';

vi.mock('@/lib/ipc', async () => {
  const { createIpcMock } = await import('@/test/ipc-mock');
  return createIpcMock();
});

const audit = auditFixture;

describe('AuditDialog', () => {
  beforeEach(() => {
    vi.mocked(getAudit).mockResolvedValue(audit);
    vi.mocked(getRunJobs).mockResolvedValue({
      items: [
        {
          jobId: 'job-1',
          title: 'Senior Rust Engineer',
          company: 'Acme',
          location: 'Remote - Brazil',
          workModality: 'Remote',
          url: 'https://example.com/job',
          evaluationStatus: 'APPROVED',
          matchScore: 82,
          rejectionReason: null,
          provider: 'deepseek',
          model: 'deepseek-flash',
        },
      ],
      total: 1,
      limit: 10,
      offset: 0,
    });
  });

  it('reconciles run totals, rejection reasons, and source failures', async () => {
    render(
      <FeedbackProvider>
        <AuditDialog runId="run-1" open onClose={() => {}} />
      </FeedbackProvider>,
    );

    expect(await screen.findByText('PARTIAL')).toBeInTheDocument();
    expect(screen.getByText('7')).toBeInTheDocument();
    expect(screen.getAllByText('2').length).toBeGreaterThan(0);
    expect(screen.getByText('SENIORITY_NOT_ELIGIBLE')).toBeInTheDocument();
    expect(screen.getByText('jobspy: timeout')).toBeInTheDocument();
    expect(screen.getByText('50.0%')).toBeInTheDocument();
    expect(await screen.findByText('Senior Rust Engineer')).toBeInTheDocument();
    await waitFor(() => expect(getAudit).toHaveBeenCalledWith('run-1'));
    await waitFor(() => expect(getRunJobs).toHaveBeenCalledWith('run-1', 10, 0));
  });

  it('lists the run jobs and paginates them', async () => {
    vi.mocked(getRunJobs).mockResolvedValue({
      items: [
        {
          jobId: 'job-1',
          title: 'Senior Rust Engineer',
          company: 'Acme',
          location: 'Remote - Brazil',
          workModality: 'Remote',
          url: 'https://example.com/job',
          evaluationStatus: 'REJECTED',
          matchScore: null,
          rejectionReason: 'seniority not eligible',
          provider: null,
          model: null,
        },
      ],
      total: 25,
      limit: 10,
      offset: 0,
    });

    render(
      <FeedbackProvider>
        <AuditDialog runId="run-1" open onClose={() => {}} />
      </FeedbackProvider>,
    );

    expect(await screen.findByText('Senior Rust Engineer')).toBeInTheDocument();
    expect(screen.getByText('seniority not eligible')).toBeInTheDocument();
    expect(screen.getByText('Jobs found (25)')).toBeInTheDocument();
    expect(screen.getByText('Page 1 of 3')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    await waitFor(() => expect(getRunJobs).toHaveBeenLastCalledWith('run-1', 10, 10));
  });

  it('shows an error instead of stale audit data', async () => {
    vi.mocked(getAudit).mockRejectedValue({ code: 'DB_ERROR', message: 'database unavailable' });
    render(
      <FeedbackProvider>
        <AuditDialog runId="run-1" open onClose={() => {}} />
      </FeedbackProvider>,
    );

    expect(await screen.findByText(/database unavailable/)).toBeInTheDocument();
  });
});
