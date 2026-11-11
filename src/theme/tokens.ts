export const tokens = {
  accent: '#7c8cff',
  accentSecondary: '#5eead4',
  darkSurface: '#0b0f19',
  darkPaper: '#121827',
  lightSurface: '#f4f6fb',
  lightPaper: '#ffffff',
  radiusCard: 18,
  radiusControl: 10,
} as const;

export function scoreTone(score: number | null | undefined) {
  if (score === null || score === undefined) return 'default' as const;
  if (score >= 90) return 'success' as const;
  if (score >= 70) return 'primary' as const;
  if (score >= 50) return 'warning' as const;
  return 'error' as const;
}
