import { useCallback, useEffect, useState } from 'react';

import { safeMessage } from '@/lib/errors';
import { exportAnalyticsCsv, getAnalytics } from '@/lib/ipc';
import type { AnalyticsPayload, AnalyticsScope, ExportResult } from '@/lib/types';

type Status = 'loading' | 'ready' | 'empty' | 'no-gap' | 'error';

export function useAnalyticsViewModel() {
  const [scope, setScope] = useState<AnalyticsScope>('LAST_7_DAYS');
  const [status, setStatus] = useState<Status>('loading');
  const [payload, setPayload] = useState<AnalyticsPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);

  const refresh = useCallback(async () => {
    setStatus('loading');
    setError(null);
    try {
      const value = await getAnalytics({ scope });
      setPayload(value);
      setStatus(
        value.denominator === 0 ? 'empty' : value.skill_gap.length === 0 ? 'no-gap' : 'ready',
      );
    } catch (cause) {
      setPayload(null);
      setStatus('error');
      setError(safeMessage(cause));
    }
  }, [scope]);

  const exportCsv = useCallback(
    async (destination: string): Promise<ExportResult> => {
      setExporting(true);
      try {
        return await exportAnalyticsCsv({
          scope,
          run_id: payload?.run_id ?? null,
          destination,
        });
      } finally {
        setExporting(false);
      }
    },
    [payload?.run_id, scope],
  );

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return {
    scope,
    setScope,
    status,
    payload,
    error,
    exporting,
    refresh,
    exportCsv,
  };
}
