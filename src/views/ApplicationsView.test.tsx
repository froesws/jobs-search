import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getJobs } from '../lib/ipc';
import { appliedJobFixture } from '../test/fixtures';
import { ApplicationsView } from './ApplicationsView';

vi.mock('../lib/ipc', async () => {
  const { createIpcMock } = await import('../test/ipc-mock');
  return createIpcMock();
});

const applied = appliedJobFixture;

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

    expect(await screen.findByText('Senior Rust Engineer')).toBeInTheDocument();
    expect(screen.getByText('Acme')).toBeInTheDocument();
    expect(screen.getAllByText('3/4').length).toBeGreaterThan(0);
    expect(screen.getByText(/Nov/)).toBeInTheDocument();
    await waitFor(() => expect(getJobs).toHaveBeenCalledWith('APPLICATIONS', 12, 0));
  });
});
