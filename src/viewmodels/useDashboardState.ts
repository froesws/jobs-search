import { useCallback, useState } from 'react';

import { DEFAULT_FILTERS } from '@/lib/types';
import type { SearchFilters } from '@/lib/types';

export type DashboardTab = 'NEW_JOBS' | 'APPLICATIONS' | 'PROFILE' | 'ANALYTICS';

export function useDashboardState() {
  const [tab, setTab] = useState<DashboardTab>('NEW_JOBS');
  const [filters, setFiltersState] = useState<SearchFilters>(DEFAULT_FILTERS);
  const [selectedJobId, setSelectedJobId] = useState<string | null>(null);

  const setFilters = useCallback((next: Partial<SearchFilters>) => {
    setFiltersState((current) => ({ ...current, ...next }));
  }, []);

  const openJob = useCallback((jobId: string) => setSelectedJobId(jobId), []);
  const closeJob = useCallback(() => setSelectedJobId(null), []);

  return {
    tab,
    setTab,
    filters,
    setFilters,
    selectedJobId,
    openJob,
    closeJob,
  };
}
