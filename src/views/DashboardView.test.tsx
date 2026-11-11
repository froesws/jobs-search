import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import App from '../App';
import { applyJob, getActiveProvider, getJob, getJobs, getProfile, listResumes } from '../lib/ipc';
import { emptyPage, jobDetailFixture, jobFixture, profileFixture } from '../test/fixtures';

vi.mock('../lib/ipc', async () => {
  const { createIpcMock } = await import('../test/ipc-mock');
  return createIpcMock();
});

const summary = jobFixture;
const detail = jobDetailFixture;

describe('dashboard application flow', () => {
  beforeEach(() => {
    vi.mocked(getActiveProvider).mockResolvedValue({
      provider: 'deepseek',
      model: 'deepseek-chat',
      fallback_enabled: false,
    });
    vi.mocked(listResumes).mockResolvedValue([]);
    vi.mocked(getProfile).mockResolvedValue(profileFixture);
    vi.mocked(getJob).mockResolvedValue(detail);
    vi.mocked(applyJob).mockResolvedValue({
      job_id: 'j1',
      lifecycle_state: 'APPLIED',
      applied_at: '2026-11-10T12:00:00Z',
      approval_source: 'automatic',
      evaluation_snapshot_json: '{}',
    });
  });

  it('applies a job, closes the drawer, and refreshes it out of New Jobs', async () => {
    vi.mocked(getJobs)
      .mockResolvedValueOnce({
        items: [summary],
        total: 1,
        limit: 12,
        offset: 0,
        tab: 'NEW_JOBS',
      })
      .mockResolvedValue(emptyPage);

    render(<App />);
    fireEvent.click(await screen.findByText('Senior Rust Engineer'));
    fireEvent.click(await screen.findByRole('button', { name: 'I Applied' }));

    await waitFor(() => expect(applyJob).toHaveBeenCalledWith(jobFixture.job_id));
    expect(await screen.findByText('No approved jobs yet')).toBeInTheDocument();
  });
});
