import { describe, expect, it } from 'vitest';

import { normalizeFilters, validateFilters } from '@/lib/filters';
import { DEFAULT_FILTERS } from '@/lib/types';

describe('search filters', () => {
  it('normalizes whitespace and removes duplicate selections', () => {
    const filters = normalizeFilters({
      ...DEFAULT_FILTERS,
      stack_keyword: '  Rust  ',
      seniorities: ['Junior', 'Junior', 'Pleno'],
      modalities: ['Remote', 'Remote'],
    });
    expect(filters.stack_keyword).toBe('Rust');
    expect(filters.seniorities).toEqual(['Junior', 'Pleno']);
    expect(filters.modalities).toEqual(['Remote']);
  });

  it('rejects an overlong stack keyword and unknown modality', () => {
    expect(
      validateFilters({
        ...DEFAULT_FILTERS,
        stack_keyword: 'x'.repeat(65),
      }),
    ).toContain('64 characters');

    expect(
      validateFilters({
        ...DEFAULT_FILTERS,
        modalities: ['Unknown'],
      }),
    ).toContain('Remote');
  });

  it('accepts the default filters and all modalities encoded as empty', () => {
    expect(validateFilters(DEFAULT_FILTERS)).toBeNull();
    expect(
      validateFilters({
        ...DEFAULT_FILTERS,
        stack_keyword: 'Rust',
        seniorities: ['Pleno'],
        modalities: [],
      }),
    ).toBeNull();
  });
});
