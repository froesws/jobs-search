import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { FeedbackProvider } from '../components/FeedbackProvider';
import { getAnalytics } from '../lib/ipc';
import type { AnalyticsPayload, SkillGapMetric } from '../lib/types';
import { AnalyticsView } from './AnalyticsView';

vi.mock('../lib/ipc', () => ({
  getAnalytics: vi.fn(),
  exportAnalyticsCsv: vi.fn(),
}));

function metrics(count: number): SkillGapMetric[] {
  return Array.from({ length: count }, (_, index) => ({
    skill: `skill-${index}`,
    display_name: `Skill ${index}`,
    demand_count: count - index,
    demand_percentage: 100 - index * 0.5,
    rank: index + 1,
  }));
}

const payload: AnalyticsPayload = {
  scope: 'LAST_7_DAYS',
  run_id: null,
  denominator: 1000,
  last_updated: '2026-11-10T00:00:00Z',
  stale: false,
  skill_gap: metrics(80),
  funnel: {
    ingested: 1000,
    duplicates: 50,
    rejected_by_reason: {},
    pending: 10,
    approved: 900,
    error: 0,
    applied: 100,
    ignored: 20,
    approval_rate: 98.9,
    application_rate: 11.1,
    source_failures: [],
    source_breakdown: {},
    provider_breakdown: {},
  },
  score_distribution: [],
  llm_metrics: {
    call_count: 1000,
    success_count: 990,
    error_count: 10,
    fallback_count: 5,
    p50_latency_ms: 100,
    p95_latency_ms: 300,
    by_provider: [],
  },
};

describe('AnalyticsView performance', () => {
  beforeEach(() => {
    vi.mocked(getAnalytics).mockResolvedValue(payload);
  });

  it('renders a large local summary within a loose regression budget', async () => {
    const started = performance.now();
    render(
      <FeedbackProvider>
        <AnalyticsView />
      </FeedbackProvider>,
    );
    await screen.findByText('Skill 0');
    const elapsed = performance.now() - started;

    // The strict 100 ms target is measured manually on the reference machine.
    expect(elapsed).toBeLessThan(2000);
    expect(screen.getByText('Skill 79')).toBeInTheDocument();
  });
});
