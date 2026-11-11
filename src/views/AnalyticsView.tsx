import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Typography from '@mui/material/Typography';
import { save } from '@tauri-apps/plugin-dialog';
import { useState } from 'react';

import { safeMessage } from '@/lib/errors';
import type { AnalyticsScope } from '@/lib/types';
import { useFeedback } from '@/components/FeedbackProvider';
import { useAnalyticsViewModel } from '@/viewmodels/useAnalyticsViewModel';
import { EmptyState } from '@/components/EmptyState';
import { ErrorState } from '@/components/ErrorState';
import { FunnelCards } from '@/components/FunnelCards';
import { LlmMetricsTable } from '@/components/LlmMetricsTable';
import { LoadingState } from '@/components/LoadingState';
import { Metric } from '@/components/Metric';
import { ScoreDistribution } from '@/components/ScoreDistribution';
import { SkillGapList } from '@/components/SkillGapList';

export function AnalyticsView() {
  const analytics = useAnalyticsViewModel();
  const feedback = useFeedback();
  const payload = analytics.payload;
  const [exportPath, setExportPath] = useState<string | null>(null);

  const chooseExport = async () => {
    try {
      const stamp = new Date().toISOString().slice(0, 16).replace(/[:T]/g, '-');
      const selected = await save({
        defaultPath: `jobs-search-analytics-${stamp}.csv`,
        filters: [{ name: 'CSV', extensions: ['csv'] }],
      });
      if (selected) setExportPath(selected);
    } catch (error) {
      feedback.notify(safeMessage(error), 'error', true);
    }
  };

  const confirmExport = async () => {
    if (!exportPath) return;
    try {
      const result = await analytics.exportCsv(exportPath);
      feedback.notify(`Exported ${result.rows} job rows to ${result.path}.`, 'success');
      setExportPath(null);
    } catch (error) {
      feedback.notify(safeMessage(error), 'error', true);
    }
  };

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
        <Button variant="outlined" onClick={() => void chooseExport()}>
          Export CSV
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

      <Dialog
        open={exportPath !== null}
        onClose={() => setExportPath(null)}
        maxWidth="sm"
        fullWidth
      >
        <DialogTitle>Export sensitive analytics data?</DialogTitle>
        <DialogContent>
          <Alert severity="warning" variant="outlined" sx={{ mb: 2 }}>
            This CSV contains job descriptions, URLs, scores, and keywords. It stays local and is
            never uploaded. Store it only in a trusted location.
          </Alert>
          <Typography variant="body2" color="text.secondary">
            Destination: {exportPath}
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setExportPath(null)}>Cancel</Button>
          <Button
            variant="contained"
            onClick={() => void confirmExport()}
            disabled={analytics.exporting}
          >
            {analytics.exporting ? 'Exporting…' : 'Export current view'}
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}
