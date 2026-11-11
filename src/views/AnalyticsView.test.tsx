import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { save } from '@tauri-apps/plugin-dialog';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { FeedbackProvider } from '../components/FeedbackProvider';
import { exportAnalyticsCsv, getAnalytics } from '../lib/ipc';
import type { AnalyticsPayload } from '../lib/types';
import { AnalyticsView } from './AnalyticsView';

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn(),
}));

vi.mock('../lib/ipc', () => ({
  getAnalytics: vi.fn(),
  exportAnalyticsCsv: vi.fn(),
}));

const payload: AnalyticsPayload = {
  scope: 'LAST_7_DAYS',
  run_id: null,
  denominator: 100,
  last_updated: '2026-11-10T00:00:00Z',
  stale: true,
  skill_gap: [
    {
      skill: 'kubernetes',
      display_name: 'Kubernetes',
      demand_count: 50,
      demand_percentage: 50,
      rank: 1,
    },
  ],
  funnel: {
    ingested: 100,
    duplicates: 0,
    rejected_by_reason: {},
    pending: 0,
    approved: 100,
    error: 0,
    applied: 0,
    ignored: 0,
    approval_rate: 100,
    application_rate: 0,
    source_failures: [],
    source_breakdown: {},
    provider_breakdown: {},
  },
  score_distribution: [{ range: '80-89', count: 100 }],
  llm_metrics: {
    call_count: 1,
    success_count: 1,
    error_count: 0,
    fallback_count: 0,
    p50_latency_ms: 100,
    p95_latency_ms: 100,
    by_provider: [],
  },
};

describe('AnalyticsView trust and export', () => {
  beforeEach(() => {
    vi.mocked(getAnalytics).mockResolvedValue(payload);
    vi.mocked(exportAnalyticsCsv).mockResolvedValue({
      path: '/tmp/analytics.csv',
      rows: 100,
      bytes: 1024,
    });
    vi.mocked(save).mockResolvedValue('/tmp/analytics.csv');
  });

  it('shows scope, denominator, stale flag, and the percentage definition', async () => {
    render(
      <FeedbackProvider>
        <AnalyticsView />
      </FeedbackProvider>,
    );

    expect(await screen.findByText('Jobs in scope')).toBeInTheDocument();
    expect(screen.getAllByText('100').length).toBeGreaterThan(0);
    expect(screen.getByText('Missing count ÷ jobs in scope × 100, 1 decimal')).toBeInTheDocument();
    expect(screen.getByText('Stale evaluations included')).toBeInTheDocument();
  });

  it('requires a sensitive-data warning before exporting', async () => {
    render(
      <FeedbackProvider>
        <AnalyticsView />
      </FeedbackProvider>,
    );
    await screen.findByText('Jobs in scope');

    fireEvent.click(screen.getByRole('button', { name: 'Export CSV' }));
    expect(await screen.findByText('Export sensitive analytics data?')).toBeInTheDocument();
    expect(screen.getByText(/This CSV contains job descriptions/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Export current view' }));
    await waitFor(() =>
      expect(exportAnalyticsCsv).toHaveBeenCalledWith({
        scope: 'LAST_7_DAYS',
        run_id: null,
        destination: '/tmp/analytics.csv',
      }),
    );
  });

  it('shows an error state instead of stale analytics', async () => {
    vi.mocked(getAnalytics).mockRejectedValue({
      code: 'DB_ERROR',
      message: 'database unavailable',
    });
    render(
      <FeedbackProvider>
        <AnalyticsView />
      </FeedbackProvider>,
    );

    expect(await screen.findByText(/database unavailable/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
  });
});
