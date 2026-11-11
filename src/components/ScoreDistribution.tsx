import Box from '@mui/material/Box';
import Paper from '@mui/material/Paper';
import Typography from '@mui/material/Typography';
import { BarChart } from '@mui/x-charts/BarChart';

import type { ScoreBucket } from '../lib/types';

export function ScoreDistribution({ buckets }: { buckets: ScoreBucket[] }) {
  return (
    <Paper sx={{ p: 2.5, display: 'grid', gap: 1 }}>
      <Typography variant="h2">Score distribution</Typography>
      {buckets.length === 0 ? (
        <Typography color="text.secondary">No evaluated scores in this scope.</Typography>
      ) : (
        <Box sx={{ width: '100%', height: 260 }}>
          <BarChart
            xAxis={[{ scaleType: 'band', data: buckets.map((bucket) => bucket.range) }]}
            series={[
              { data: buckets.map((bucket) => bucket.count), label: 'Jobs', color: '#7c8cff' },
            ]}
            margin={{ top: 16, right: 16, bottom: 32, left: 40 }}
            grid={{ horizontal: true }}
          />
        </Box>
      )}
    </Paper>
  );
}
