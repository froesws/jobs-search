import Box from '@mui/material/Box';
import Paper from '@mui/material/Paper';
import Typography from '@mui/material/Typography';

import type { SkillGapMetric } from '@/lib/types';
import { SkillGapBar } from '@/components/SkillGapBar';

export function SkillGapList({ metrics }: { metrics: SkillGapMetric[] }) {
  return (
    <Paper sx={{ p: 2.5, display: 'grid', gap: 2 }}>
      <Typography variant="h2">Skill Gap</Typography>
      {metrics.length === 0 ? (
        <Typography color="text.secondary">
          No missing skills in this scope. Every extracted requirement is covered or excluded.
        </Typography>
      ) : (
        <Box sx={{ display: 'grid', gap: 1.75 }}>
          {metrics.map((metric) => (
            <Box
              key={metric.skill}
              sx={{ display: 'grid', gridTemplateColumns: '28px 1fr', gap: 1.5 }}
            >
              <Typography variant="caption" color="text.secondary" sx={{ pt: 0.25 }}>
                #{metric.rank}
              </Typography>
              <SkillGapBar metric={metric} />
            </Box>
          ))}
        </Box>
      )}
    </Paper>
  );
}
