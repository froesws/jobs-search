import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { save } from '@tauri-apps/plugin-dialog';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { FeedbackProvider } from '@/components/FeedbackProvider';
import { exportAnalyticsCsv, getAnalytics } from '@/lib/ipc';
import { analyticsPayloadFixture } from '@/test/fixtures';
import { AnalyticsView } from '@/views/AnalyticsView';

vi.mock('@tauri-apps/plugin-dialog', () => ({
  save: vi.fn(),
}));

vi.mock('@/lib/ipc', async () => {
  const { createIpcMock } = await import('@/test/ipc-mock');
  return createIpcMock();
});

const payload = analyticsPayloadFixture;

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
