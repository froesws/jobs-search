import { describe, expect, it } from 'vitest';

import { ghostTransform, rowIndexAt } from '@/components/providerDrag';

const rows = [
  { top: 0, bottom: 40 },
  { top: 50, bottom: 90 },
  { top: 100, bottom: 140 },
];

describe('ghostTransform', () => {
  it('offsets the ghost down and right from the pointer', () => {
    expect(ghostTransform({ x: 120, y: 80 })).toBe('translate(134px, 62px)');
  });

  it('keeps the offset near the viewport origin', () => {
    expect(ghostTransform({ x: 0, y: 0 })).toBe('translate(14px, -18px)');
  });
});

describe('rowIndexAt', () => {
  it('returns the row that contains the pointer', () => {
    expect(rowIndexAt(rows, 0)).toBe(0);
    expect(rowIndexAt(rows, 20)).toBe(0);
    expect(rowIndexAt(rows, 50)).toBe(1);
    expect(rowIndexAt(rows, 140)).toBe(2);
  });

  it('resolves a gap to the nearest row by center distance', () => {
    expect(rowIndexAt(rows, 44)).toBe(0);
    expect(rowIndexAt(rows, 46)).toBe(1);
    expect(rowIndexAt(rows, 94)).toBe(1);
    expect(rowIndexAt(rows, 96)).toBe(2);
  });

  it('clamps above the first and below the last row', () => {
    expect(rowIndexAt(rows, -50)).toBe(0);
    expect(rowIndexAt(rows, 9_999)).toBe(2);
  });

  it('returns null when there are no rows', () => {
    expect(rowIndexAt([], 10)).toBeNull();
  });
});
