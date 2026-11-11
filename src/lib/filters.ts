import { DEFAULT_FILTERS } from '@/lib/types';
import type { SearchFilters, SearchTimeframe, WorkModality } from '@/lib/types';

const TIMEFRAMES: SearchTimeframe[] = ['H24', 'D7', 'D30'];
const MODALITIES: WorkModality[] = ['Remote', 'Hybrid', 'Onsite'];

export function normalizeFilters(filters: SearchFilters): SearchFilters {
  return {
    ...DEFAULT_FILTERS,
    ...filters,
    titles: filters.titles.map((value) => value.trim()).filter(Boolean),
    stack_keyword: filters.stack_keyword.trim(),
    seniorities: [...new Set(filters.seniorities)],
    modalities: [...new Set(filters.modalities)],
    source_ids: [...new Set(filters.source_ids)],
  };
}

/** Return a user-facing validation message, or null when the filters can run. */
export function validateFilters(filters: SearchFilters): string | null {
  if (!TIMEFRAMES.includes(filters.timeframe)) {
    return 'Choose a valid posting timeframe.';
  }
  if (filters.stack_keyword.trim().length > 64) {
    return 'The stack keyword must be 64 characters or fewer.';
  }
  if (filters.seniorities.length > 5) {
    return 'Select at most five seniority levels.';
  }
  if (filters.modalities.some((modality) => !MODALITIES.includes(modality))) {
    return 'Choose Remote, Hybrid, On-site, or All.';
  }
  return null;
}
