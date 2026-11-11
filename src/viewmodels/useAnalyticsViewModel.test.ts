import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getAnalytics } from '@/lib/ipc';
import type { AnalyticsPayload } from '@/lib/types';
import { useAnalyticsViewModel } from '@/viewmodels/useAnalyticsViewModel';

vi.mock('@/lib/ipc', async () => {
  const { createIpcMock } = await import('@/test/ipc-mock');
  return createIpcMock();
});

function payload(overrides: Partial<AnalyticsPayload> = {}): AnalyticsPayload {
  return {
    scope: 'LAST_7_DAYS',
    run_id: null,
    denominator: 100,
    last_updated: '2026-11-10T00:00:00Z',
    stale: false,
    skill_gap: [
      {
        skill: 'kubernetes',
        display_name: 'Kubernetes',
        demand_count: 50,
        demand_percentage: 50,
        rank: 1,
      },
    ],
    funnel: {
      ingested: 100,
      duplicates: 0,
      rejected_by_reason: {},
      pending: 0,
      approved: 100,
      error: 0,
      applied: 0,
      ignored: 0,
      approval_rate: 100,
      application_rate: 0,
      source_failures: [],
      source_breakdown: {},
      provider_breakdown: {},
    },
    score_distribution: [],
    llm_metrics: {
      call_count: 0,
      success_count: 0,
      error_count: 0,
      fallback_count: 0,
      p50_latency_ms: 0,
      p95_latency_ms: 0,
      by_provider: [],
    },
    ...overrides,
  };
}

describe('useAnalyticsViewModel', () => {
  beforeEach(() => {
    vi.mocked(getAnalytics).mockReset();
  });

  it('loads the default Last 7 Days scope', async () => {
    vi.mocked(getAnalytics).mockResolvedValue(payload());
    const { result } = renderHook(() => useAnalyticsViewModel());

    await waitFor(() => expect(result.current.status).toBe('ready'));
    expect(result.current.payload?.denominator).toBe(100);
    expect(getAnalytics).toHaveBeenCalledWith({ scope: 'LAST_7_DAYS' });
  });

  it('reloads when the scope changes', async () => {
    vi.mocked(getAnalytics).mockResolvedValue(payload());
    const { result } = renderHook(() => useAnalyticsViewModel());
    await waitFor(() => expect(result.current.status).toBe('ready'));

    act(() => result.current.setScope('CURRENT_RUN'));
    await waitFor(() => expect(getAnalytics).toHaveBeenCalledWith({ scope: 'CURRENT_RUN' }));
  });

  it('distinguishes empty scope from zero-gap scope', async () => {
    vi.mocked(getAnalytics).mockResolvedValueOnce(payload({ denominator: 0, skill_gap: [] }));
    const empty = renderHook(() => useAnalyticsViewModel());
    await waitFor(() => expect(empty.result.current.status).toBe('empty'));

    vi.mocked(getAnalytics).mockResolvedValueOnce(payload({ skill_gap: [] }));
    const noGap = renderHook(() => useAnalyticsViewModel());
    await waitFor(() => expect(noGap.result.current.status).toBe('no-gap'));
  });

  it('surfaces an analytics error', async () => {
    vi.mocked(getAnalytics).mockRejectedValue({
      code: 'DB_ERROR',
      message: 'database unavailable',
    });
    const { result } = renderHook(() => useAnalyticsViewModel());
    await waitFor(() => expect(result.current.status).toBe('error'));
    expect(result.current.error).toBe('database unavailable');
  });
});
