import type { JobSummary, WorkModality } from '@/lib/types';

export function scoreLabel(score: number | null | undefined): string {
  return score === null || score === undefined ? '—' : String(Math.round(score));
}

export function ratioLabel(
  job: Pick<JobSummary, 'matching_items_count' | 'total_requirements_count'>,
) {
  const matched = job.matching_items_count;
  const total = job.total_requirements_count;
  if (matched === null || total === null) return 'Not evaluated';
  if (total === 0) return 'No requirements';
  return `${matched}/${total}`;
}

export function formatDate(value: string | null | undefined): string {
  if (!value) return 'Unknown';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return 'Unknown';
  return new Intl.DateTimeFormat('en', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
  }).format(date);
}

export function postingAge(postedAt: string | null): string {
  if (!postedAt) return 'Age unknown';
  const date = new Date(postedAt);
  if (Number.isNaN(date.getTime())) return 'Age unknown';
  const days = Math.max(0, Math.round((Date.now() - date.getTime()) / 86_400_000));
  if (days === 0) return 'Today';
  if (days === 1) return '1 day ago';
  return `${days} days ago`;
}

export function modalityLabel(value: WorkModality | string): string {
  if (value === 'Onsite') return 'On-site';
  return value;
}

export function safeExternalUrl(value: string): string | null {
  try {
    const url = new URL(value);
    if (url.protocol !== 'http:' && url.protocol !== 'https:') return null;
    return url.toString();
  } catch {
    return null;
  }
}

export function statusLabel(status: string | null): string {
  if (!status) return 'Discovered';
  return status.charAt(0) + status.slice(1).toLowerCase();
}
