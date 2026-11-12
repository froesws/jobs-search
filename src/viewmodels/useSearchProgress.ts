import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';

import type { SearchProgress, SearchStage } from '@/lib/types';

/** Tauri event emitted by the backend while a search run is active. */
export const SEARCH_PROGRESS_EVENT = 'search://progress';

const STAGES: SearchStage[] = ['collecting', 'evaluating'];

function isCount(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0;
}

/** Validate an untrusted event payload; malformed values are ignored. */
export function parseSearchProgress(payload: unknown): SearchProgress | null {
  if (!payload || typeof payload !== 'object') return null;
  const candidate = payload as Record<string, unknown>;
  if (!STAGES.includes(candidate.stage as SearchStage)) return null;
  if (
    !isCount(candidate.found) ||
    !isCount(candidate.evaluated) ||
    !isCount(candidate.total) ||
    !isCount(candidate.failed)
  ) {
    return null;
  }
  return {
    stage: candidate.stage as SearchStage,
    found: candidate.found,
    evaluated: candidate.evaluated,
    total: candidate.total,
    failed: candidate.failed,
  };
}

/**
 * Subscribe to backend progress while a run is active. Returns `null` before
 * the first event, after the run ends, and when the subscription fails.
 */
export function useSearchProgress(active: boolean): SearchProgress | null {
  const [progress, setProgress] = useState<SearchProgress | null>(null);

  useEffect(() => {
    if (!active) {
      setProgress(null);
      return;
    }
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    listen<unknown>(SEARCH_PROGRESS_EVENT, (event) => {
      const next = parseSearchProgress(event.payload);
      if (!cancelled && next) setProgress(next);
    })
      .then((stop) => {
        if (cancelled) {
          stop();
        } else {
          unlisten = stop;
        }
      })
      .catch(() => {
        if (!cancelled) setProgress(null);
      });

    return () => {
      cancelled = true;
      unlisten?.();
      unlisten = undefined;
      setProgress(null);
    };
  }, [active]);

  return active ? progress : null;
}
