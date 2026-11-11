import { describe, expect, it } from 'vitest';

import {
  MODALITY_PANEL_VALUES,
  SENIORITY_PANEL_VALUES,
  normalizeFilters,
  toggleModality,
  toggleSeniority,
  validateFilters,
} from '@/lib/filters';
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

  it('toggles only the four seniority panel values', () => {
    expect(SENIORITY_PANEL_VALUES).toEqual(['Trainee', 'Junior', 'Pleno', 'Senior']);
    expect(toggleSeniority([], 'Trainee')).toEqual(['Trainee']);
    expect(toggleSeniority(['Trainee'], 'Junior')).toEqual(['Trainee', 'Junior']);
    expect(toggleSeniority(['Trainee', 'Junior'], 'Trainee')).toEqual(['Junior']);
    expect(toggleSeniority(['Intern'], 'Intern')).toEqual([]);
  });

  it('toggles only the three work-type panel values', () => {
    expect(MODALITY_PANEL_VALUES).toEqual(['Onsite', 'Hybrid', 'Remote']);
    expect(toggleModality([], 'Remote')).toEqual(['Remote']);
    expect(toggleModality(['Remote'], 'Hybrid')).toEqual(['Hybrid', 'Remote']);
    expect(toggleModality(['Remote', 'Hybrid'], 'Remote')).toEqual(['Hybrid']);
    expect(toggleModality(['Hybrid'], 'Hybrid')).toEqual([]);
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
