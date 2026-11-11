import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getJob } from '@/lib/ipc';
import type { JobDetail } from '@/lib/types';
import { useJobDetailViewModel } from '@/viewmodels/useJobDetailViewModel';

vi.mock('@/lib/ipc', async () => {
  const { createIpcMock } = await import('@/test/ipc-mock');
  return createIpcMock();
});

const detail = {
  job_id: 'j1',
  title: 'Rust Engineer',
  company: 'Acme',
  location: 'Remote',
  work_modality: 'Remote',
  url: 'https://example.com/1',
  posted_at: null,
  evaluation_status: 'ERROR',
  lifecycle_state: 'DISCOVERED',
  match_score: null,
  matching_items_count: null,
  total_requirements_count: null,
  summary: '',
  needs_review: false,
  applied_at: null,
  description_clean: 'Rust',
  requirements: [],
  matched_keywords: [],
  missing_keywords: [],
  hard_filter_rejections: [],
  source_lineage: [],
  score_breakdown: null,
  provider: 'deepseek',
  model: 'deepseek-chat',
  error_category: 'RATE_LIMIT',
} as JobDetail;

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
