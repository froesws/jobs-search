import { useCallback, useState } from 'react';

import { safeMessage } from '@/lib/errors';
import { getJob } from '@/lib/ipc';
import type { JobDetail } from '@/lib/types';

type Status = 'idle' | 'loading' | 'ready' | 'error';

export function useJobDetailViewModel() {
  const [status, setStatus] = useState<Status>('idle');
  const [job, setJob] = useState<JobDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const load = useCallback(async (jobId: string) => {
    setStatus('loading');
    setError(null);
    try {
      const detail = await getJob(jobId);
      setJob(detail);
      setStatus('ready');
    } catch (cause) {
      setJob(null);
      setStatus('error');
      setError(safeMessage(cause));
    }
  }, []);

  const open = useCallback(
    (jobId: string) => {
      setSelectedId(jobId);
      void load(jobId);
    },
    [load],
  );

  const close = useCallback(() => {
    setSelectedId(null);
    setJob(null);
    setStatus('idle');
    setError(null);
  }, []);

  const retry = useCallback(() => {
    if (selectedId) void load(selectedId);
  }, [load, selectedId]);

  return {
    status,
    job,
    error,
    selectedId,
    isDrawerOpen: selectedId !== null,
    open,
    close,
    retry,
  };
}
