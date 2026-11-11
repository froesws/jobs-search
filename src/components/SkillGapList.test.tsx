import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import type { SkillGapMetric } from '@/lib/types';
import { SkillGapList } from '@/components/SkillGapList';

const metrics: SkillGapMetric[] = [
  {
    skill: 'kubernetes',
    display_name: 'Kubernetes',
    demand_count: 50,
    demand_percentage: 50,
    rank: 1,
  },
  {
    skill: 'grpc',
    display_name: 'gRPC',
    demand_count: 20,
    demand_percentage: 20,
    rank: 2,
  },
];

describe('SkillGapList', () => {
  it('renders rank, demand, and percentage with accessible progress bars', () => {
    render(<SkillGapList metrics={metrics} />);

    expect(screen.getByText('Kubernetes')).toBeInTheDocument();
    expect(screen.getByText('#1')).toBeInTheDocument();
    expect(screen.getByText('50 jobs · 50.0%')).toBeInTheDocument();
    expect(screen.getByLabelText('Kubernetes: 50 jobs missing, 50.0 percent')).toBeInTheDocument();
    expect(screen.getByText('gRPC')).toBeInTheDocument();
  });

  it('shows the zero-gap message', () => {
    render(<SkillGapList metrics={[]} />);
    expect(screen.getByText(/No missing skills in this scope/)).toBeInTheDocument();
  });
});
