import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getJobs } from '../lib/ipc';
import { useJobsViewModel } from './useJobsViewModel';

vi.mock('../lib/ipc', () => ({
  getJobs: vi.fn(),
  triggerSearch: vi.fn(),
}));

const page = {
  items: [],
  total: 0,
  limit: 12,
  offset: 0,
  tab: 'NEW_JOBS' as const,
};

describe('useJobsViewModel', () => {
  beforeEach(() => {
    vi.mocked(getJobs).mockReset();
  });

  it('moves from loading to empty', async () => {
    vi.mocked(getJobs).mockResolvedValue(page);
    const { result } = renderHook(() => useJobsViewModel('NEW_JOBS', 0));

    expect(result.current.status).toBe('loading');
    await waitFor(() => expect(result.current.status).toBe('empty'));
    expect(getJobs).toHaveBeenCalledWith('NEW_JOBS', 12, 0);
  });

  it('exposes a safe error message', async () => {
    vi.mocked(getJobs).mockRejectedValue({ code: 'DB_ERROR', message: 'database unavailable' });
    const { result } = renderHook(() => useJobsViewModel('NEW_JOBS', 0));

    await waitFor(() => expect(result.current.status).toBe('error'));
    expect(result.current.error).toBe('database unavailable');
  });
});
