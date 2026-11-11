import { describe, expect, it } from 'vitest';

import { modalityLabel, postingAge, ratioLabel, safeExternalUrl, scoreLabel } from './format';

describe('format helpers', () => {
  it('renders zero requirements as an explicit state', () => {
    expect(ratioLabel({ matching_items_count: 0, total_requirements_count: 0 })).toBe(
      'No requirements',
    );
    expect(ratioLabel({ matching_items_count: null, total_requirements_count: null })).toBe(
      'Not evaluated',
    );
    expect(ratioLabel({ matching_items_count: 3, total_requirements_count: 4 })).toBe('3/4');
  });

  it('labels missing scores and ages as unknown', () => {
    expect(scoreLabel(null)).toBe('—');
    expect(postingAge(null)).toBe('Age unknown');
    expect(modalityLabel('Onsite')).toBe('On-site');
  });

  it('only accepts http(s) external urls', () => {
    expect(safeExternalUrl('https://example.com/job')).toBe('https://example.com/job');
    expect(safeExternalUrl('javascript:alert(1)')).toBeNull();
    expect(safeExternalUrl('not a url')).toBeNull();
  });
});
