import { useCallback, useEffect, useState } from 'react';

import { safeMessage, toAppError } from '../lib/errors';
import { getJobs, triggerSearch } from '../lib/ipc';
import type { JobSummary, JobTab, SearchFilters, SearchMetrics } from '../lib/types';

const PAGE_SIZE = 12;

type Status = 'idle' | 'loading' | 'ready' | 'empty' | 'error';

interface JobsState {
  status: Status;
  items: JobSummary[];
  total: number;
  limit: number;
  offset: number;
  error: string | null;
}

const INITIAL: JobsState = {
  status: 'idle',
  items: [],
  total: 0,
  limit: PAGE_SIZE,
  offset: 0,
  error: null,
};

export function useJobsViewModel(tab: JobTab, refreshToken: number) {
  const [state, setState] = useState<JobsState>(INITIAL);
  const [offset, setOffset] = useState(0);
  const [reload, setReload] = useState(0);
  const [searching, setSearching] = useState(false);
  const [metrics, setMetrics] = useState<SearchMetrics | null>(null);

  useEffect(() => {
    setOffset(0);
  }, [tab]);

  useEffect(() => {
    let active = true;
    setState((current) => ({ ...current, status: 'loading', error: null }));
    getJobs(tab, PAGE_SIZE, offset)
      .then((page) => {
        if (!active) return;
        setState({
          status: page.items.length === 0 ? 'empty' : 'ready',
          items: page.items,
          total: page.total,
          limit: page.limit,
          offset: page.offset,
          error: null,
        });
      })
      .catch((error: unknown) => {
        if (!active) return;
        setState((current) => ({
          ...current,
          status: 'error',
          error: safeMessage(error),
        }));
      });
    return () => {
      active = false;
    };
  }, [tab, offset, reload, refreshToken]);

  const retry = useCallback(() => setReload((value) => value + 1), []);

  const nextPage = useCallback(() => {
    setOffset((value) => value + PAGE_SIZE);
  }, []);

  const previousPage = useCallback(() => {
    setOffset((value) => Math.max(0, value - PAGE_SIZE));
  }, []);

  const runSearch = useCallback(async (filters: SearchFilters) => {
    setSearching(true);
    try {
      const result = await triggerSearch(filters);
      setMetrics(result);
      setOffset(0);
      return result;
    } catch (error) {
      throw toAppError(error);
    } finally {
      setSearching(false);
    }
  }, []);

  return {
    ...state,
    searching,
    metrics,
    retry,
    nextPage,
    previousPage,
    runSearch,
    hasNext: state.offset + state.limit < state.total,
    hasPrevious: state.offset > 0,
  };
}
