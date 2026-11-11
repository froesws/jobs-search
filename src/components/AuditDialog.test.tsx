import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getAudit } from '../lib/ipc';
import type { ExecutionAudit } from '../lib/types';
import { AuditDialog } from './AuditDialog';

vi.mock('../lib/ipc', () => ({
  getAudit: vi.fn(),
}));

const audit: ExecutionAudit = {
  run_id: 'run-1',
  status: 'PARTIAL',
  started_at: '2026-11-10T10:00:00Z',
  finished_at: '2026-11-10T10:05:00Z',
  search_config: { timeframe: 'D7' },
  total_ingested: 7,
  duplicates_merged: 2,
  rejected_count: 2,
  pending_count: 1,
  approved_count: 3,
  error_count: 1,
  applied_count: 2,
  ignored_count: 1,
  approval_rate: 50,
  application_rate: 66.7,
  rejected_by_reason: { SENIORITY_NOT_ELIGIBLE: 2 },
  source_failures: ['jobspy: timeout'],
  score_distribution: { '80-89': 2 },
  source_breakdown: { jobspy: 5 },
  provider_breakdown: { 'deepseek/deepseek-chat': 4 },
};

describe('AuditDialog', () => {
  beforeEach(() => {
    vi.mocked(getAudit).mockResolvedValue(audit);
  });

  it('reconciles run totals, rejection reasons, and source failures', async () => {
    render(<AuditDialog runId="run-1" open onClose={() => {}} />);

    expect(await screen.findByText('PARTIAL')).toBeInTheDocument();
    expect(screen.getByText('7')).toBeInTheDocument();
    expect(screen.getAllByText('2').length).toBeGreaterThan(0);
    expect(screen.getByText('SENIORITY_NOT_ELIGIBLE')).toBeInTheDocument();
    expect(screen.getByText('jobspy: timeout')).toBeInTheDocument();
    expect(screen.getByText('50.0%')).toBeInTheDocument();
    await waitFor(() => expect(getAudit).toHaveBeenCalledWith('run-1'));
  });

  it('shows an error instead of stale audit data', async () => {
    vi.mocked(getAudit).mockRejectedValue({ code: 'DB_ERROR', message: 'database unavailable' });
    render(<AuditDialog runId="run-1" open onClose={() => {}} />);

    expect(await screen.findByText(/database unavailable/)).toBeInTheDocument();
  });
});
