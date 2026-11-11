import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getJob } from '@/lib/ipc';
import { useJobDetailViewModel } from '@/viewmodels/useJobDetailViewModel';
import { errorJobDetailFixture } from '@/test/fixtures';

vi.mock('@/lib/ipc', async () => {
  const { createIpcMock } = await import('@/test/ipc-mock');
  return createIpcMock();
});

const detail = errorJobDetailFixture;

describe('useJobDetailViewModel', () => {
  beforeEach(() => {
    vi.mocked(getJob).mockReset();
  });

  it('exposes an evaluation error without a score and retries', async () => {
    vi.mocked(getJob).mockRejectedValueOnce({ code: 'PROVIDER_ERROR', message: 'rate limited' });
    const { result } = renderHook(() => useJobDetailViewModel());

    result.current.open('j1');
    await waitFor(() => expect(result.current.status).toBe('error'));
    expect(result.current.error).toBe('rate limited');
    expect(result.current.job).toBeNull();

    vi.mocked(getJob).mockResolvedValueOnce(detail);
    result.current.retry();
    await waitFor(() => expect(result.current.status).toBe('ready'));
    expect(result.current.job?.match_score).toBeNull();
  });

  it('closes and clears the drawer', async () => {
    vi.mocked(getJob).mockResolvedValue(detail);
    const { result } = renderHook(() => useJobDetailViewModel());

    result.current.open('j1');
    await waitFor(() => expect(result.current.isDrawerOpen).toBe(true));
    result.current.close();
    await waitFor(() => expect(result.current.isDrawerOpen).toBe(false));
    expect(result.current.job).toBeNull();
  });
});
