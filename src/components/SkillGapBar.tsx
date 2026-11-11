import Box from '@mui/material/Box';
import LinearProgress from '@mui/material/LinearProgress';
import Typography from '@mui/material/Typography';

import type { SkillGapMetric } from '@/lib/types';

export function SkillGapBar({ metric }: { metric: SkillGapMetric }) {
  return (
    <Box sx={{ display: 'grid', gap: 0.5 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', gap: 2 }}>
        <Typography variant="body2" sx={{ fontWeight: 600 }}>
          {metric.display_name}
        </Typography>
        <Typography variant="caption" color="text.secondary">
          {metric.demand_count} jobs · {metric.demand_percentage.toFixed(1)}%
        </Typography>
      </Box>
      <LinearProgress
        variant="determinate"
        value={Math.min(100, Math.max(0, metric.demand_percentage))}
        color={metric.demand_percentage >= 50 ? 'primary' : 'secondary'}
        aria-label={`${metric.display_name}: ${metric.demand_count} jobs missing, ${metric.demand_percentage.toFixed(1)} percent`}
      />
    </Box>
  );
}
