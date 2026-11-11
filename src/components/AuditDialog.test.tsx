import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getAudit } from '../lib/ipc';
import { auditFixture } from '../test/fixtures';
import { AuditDialog } from './AuditDialog';

vi.mock('../lib/ipc', async () => {
  const { createIpcMock } = await import('../test/ipc-mock');
  return createIpcMock();
});

const audit = auditFixture;

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
