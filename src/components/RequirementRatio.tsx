import Box from '@mui/material/Box';
import LinearProgress from '@mui/material/LinearProgress';
import Typography from '@mui/material/Typography';

import { ratioLabel } from '@/lib/format';
import type { JobSummary } from '@/lib/types';

export function RequirementRatio({
  job,
  compact = false,
}: {
  job: Pick<JobSummary, 'matching_items_count' | 'total_requirements_count'>;
  compact?: boolean;
}) {
  const total = job.total_requirements_count ?? 0;
  const matched = job.matching_items_count ?? 0;
  const value = total === 0 ? 0 : (matched / total) * 100;

  return (
    <Box sx={{ minWidth: compact ? 96 : 140 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', gap: 1, mb: 0.5 }}>
        <Typography variant="caption" color="text.secondary">
          Requirements
        </Typography>
        <Typography variant="caption" sx={{ fontWeight: 700 }}>
          {ratioLabel(job)}
        </Typography>
      </Box>
      <LinearProgress
        variant="determinate"
        value={Math.min(100, Math.max(0, value))}
        color={
          total === 0 ? 'inherit' : value >= 70 ? 'success' : value >= 40 ? 'warning' : 'error'
        }
        aria-label={`${ratioLabel(job)} requirements matched`}
      />
    </Box>
  );
}
