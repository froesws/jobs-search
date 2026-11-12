import { act, renderHook, waitFor } from '@testing-library/react';
import { listen } from '@tauri-apps/api/event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { parseSearchProgress, useSearchProgress } from '@/viewmodels/useSearchProgress';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

type Handler = (event: { payload: unknown }) => void;

function subscribe() {
  let handler: Handler | undefined;
  const unlisten = vi.fn();
  vi.mocked(listen).mockImplementation((_event, callback) => {
    handler = callback as Handler;
    return Promise.resolve(unlisten);
  });
  return {
    unlisten,
    emit(payload: unknown) {
      act(() => handler?.({ payload }));
    },
  };
}

const payload = {
  stage: 'collecting' as const,
  found: 42,
  evaluated: 0,
  total: 0,
  failed: 1,
};

describe('useSearchProgress', () => {
  beforeEach(() => {
    vi.mocked(listen).mockReset();
  });

  it('subscribes when active and updates on each event', async () => {
    const bus = subscribe();
    const { result } = renderHook(() => useSearchProgress(true));

    await waitFor(() =>
      expect(listen).toHaveBeenCalledWith('search://progress', expect.any(Function)),
    );
    expect(result.current).toBeNull();

    bus.emit(payload);
    expect(result.current).toEqual(payload);

    bus.emit({ ...payload, stage: 'evaluating', evaluated: 7, total: 10 });
    expect(result.current).toEqual({
      stage: 'evaluating',
      found: 42,
      evaluated: 7,
      total: 10,
      failed: 1,
    });
  });

  it('does not subscribe while inactive', () => {
    const { result } = renderHook(() => useSearchProgress(false));
    expect(listen).not.toHaveBeenCalled();
    expect(result.current).toBeNull();
  });

  it('unsubscribes and clears when the run ends', async () => {
    const bus = subscribe();
    const { result, rerender } = renderHook(({ active }) => useSearchProgress(active), {
      initialProps: { active: true },
    });
    await waitFor(() => expect(listen).toHaveBeenCalled());
    bus.emit(payload);
    expect(result.current).toEqual(payload);

    rerender({ active: false });
    expect(bus.unlisten).toHaveBeenCalled();
    expect(result.current).toBeNull();
  });

  it('unsubscribes on unmount', async () => {
    const bus = subscribe();
    const { unmount } = renderHook(() => useSearchProgress(true));
    await waitFor(() => expect(listen).toHaveBeenCalled());
    unmount();
    expect(bus.unlisten).toHaveBeenCalled();
  });

  it('returns null when the subscription fails', async () => {
    vi.mocked(listen).mockRejectedValue(new Error('no permissions'));
    const { result } = renderHook(() => useSearchProgress(true));
    await waitFor(() => expect(listen).toHaveBeenCalled());
    expect(result.current).toBeNull();
  });

  it('ignores malformed payloads', async () => {
    const bus = subscribe();
    const { result } = renderHook(() => useSearchProgress(true));
    await waitFor(() => expect(listen).toHaveBeenCalled());

    bus.emit({ stage: 'evaluating', found: 'many' });
    bus.emit(null);
    bus.emit({ ...payload, stage: 'done' });
    expect(result.current).toBeNull();

    bus.emit(payload);
    expect(result.current).toEqual(payload);
  });
});

describe('parseSearchProgress', () => {
  it('rejects negative and non-finite counters', () => {
    expect(parseSearchProgress({ ...payload, found: -1 })).toBeNull();
    expect(parseSearchProgress({ ...payload, total: Number.NaN })).toBeNull();
    expect(parseSearchProgress({ ...payload, failed: Number.POSITIVE_INFINITY })).toBeNull();
  });
});
