import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Typography from '@mui/material/Typography';

import type { AnalyticsScope } from '../lib/types';
import { useAnalyticsViewModel } from '../viewmodels/useAnalyticsViewModel';
import { EmptyState } from '../components/EmptyState';
import { ErrorState } from '../components/ErrorState';
import { FunnelCards } from '../components/FunnelCards';
import { LlmMetricsTable } from '../components/LlmMetricsTable';
import { LoadingState } from '../components/LoadingState';
import { ScoreDistribution } from '../components/ScoreDistribution';
import { SkillGapList } from '../components/SkillGapList';

export function AnalyticsView() {
  const analytics = useAnalyticsViewModel();
  const payload = analytics.payload;

  return (
    <Box sx={{ display: 'grid', gap: 2 }}>
      <Box sx={{ display: 'flex', alignItems: 'flex-end', gap: 2, flexWrap: 'wrap' }}>
        <Box sx={{ flex: 1 }}>
          <Typography variant="h1">Analytics</Typography>
          <Typography color="text.secondary" variant="body2">
            Missing-skill demand and pipeline health, computed locally from evaluated jobs.
          </Typography>
        </Box>
        <ToggleButtonGroup
          exclusive
          size="small"
          value={analytics.scope}
          onChange={(_, value: AnalyticsScope | null) => {
            if (value) analytics.setScope(value);
          }}
          aria-label="Analytics time range"
        >
          <ToggleButton value="CURRENT_RUN">Current Run</ToggleButton>
          <ToggleButton value="LAST_7_DAYS">Last 7 Days</ToggleButton>
        </ToggleButtonGroup>
        <Button variant="outlined" onClick={() => void analytics.refresh()}>
          Refresh
        </Button>
      </Box>

      {analytics.status === 'loading' ? <LoadingState label="Computing analytics…" /> : null}
      {analytics.status === 'error' ? (
        <ErrorState
          message={analytics.error ?? 'Unknown analytics error'}
          onRetry={analytics.refresh}
        />
      ) : null}

      {analytics.status === 'empty' ? (
        <EmptyState
          title="No evaluated jobs in this scope"
          description="Run a search and let evaluation complete. Rejected, error, and incomplete evaluations are excluded from analytics."
        />
      ) : null}

      {payload && analytics.status !== 'loading' && analytics.status !== 'error' ? (
        <>
          <Box
            sx={{
              display: 'flex',
              gap: 2,
              flexWrap: 'wrap',
              alignItems: 'center',
              p: 2,
              border: '1px solid',
              borderColor: 'divider',
              borderRadius: 3,
              bgcolor: 'background.paper',
            }}
          >
            <Chip
              size="small"
              label={analytics.scope === 'CURRENT_RUN' ? 'Current Run' : 'Last 7 Days'}
              color="primary"
              variant="outlined"
            />
            <Metric label="Jobs in scope" value={String(payload.denominator)} />
            <Metric label="Last updated" value={new Date(payload.last_updated).toLocaleString()} />
            <Metric label="Percentage" value="Missing count ÷ jobs in scope × 100, 1 decimal" />
            {payload.stale ? (
              <Chip
                size="small"
                label="Stale evaluations included"
                color="warning"
                variant="outlined"
              />
            ) : null}
          </Box>

          {analytics.status === 'no-gap' ? (
            <Alert severity="success" variant="outlined">
              No missing skills in this scope. The denominator still includes {payload.denominator}{' '}
              evaluated jobs.
            </Alert>
          ) : null}

          <SkillGapList metrics={payload.skill_gap} />
          <FunnelCards funnel={payload.funnel} />
          <ScoreDistribution buckets={payload.score_distribution} />
          <LlmMetricsTable metrics={payload.llm_metrics} />
        </>
      ) : null}
    </Box>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <Box sx={{ display: 'grid', gap: 0.25, minWidth: 120 }}>
      <Typography variant="caption" color="text.secondary">
        {label}
      </Typography>
      <Typography variant="body2" sx={{ fontWeight: 700 }}>
        {value}
      </Typography>
    </Box>
  );
}
