import Box from '@mui/material/Box';
import Paper from '@mui/material/Paper';
import Typography from '@mui/material/Typography';

import type { FunnelPayload } from '../lib/types';

export function FunnelCards({ funnel }: { funnel: FunnelPayload }) {
  return (
    <Paper sx={{ p: 2.5, display: 'grid', gap: 2 }}>
      <Typography variant="h2">Funnel</Typography>
      <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(4, minmax(0, 1fr))', gap: 1.5 }}>
        <Metric label="Ingested" value={funnel.ingested} />
        <Metric label="Duplicates" value={funnel.duplicates} />
        <Metric
          label="Rejected"
          value={Object.values(funnel.rejected_by_reason).reduce((sum, count) => sum + count, 0)}
        />
        <Metric label="Pending" value={funnel.pending} />
        <Metric label="Approved" value={funnel.approved} />
        <Metric label="Applied" value={funnel.applied} />
        <Metric label="Ignored" value={funnel.ignored} />
        <Metric label="Approval rate" value={`${funnel.approval_rate.toFixed(1)}%`} />
        <Metric label="Application rate" value={`${funnel.application_rate.toFixed(1)}%`} />
      </Box>
      <Box sx={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 2 }}>
        <Breakdown title="By source" values={funnel.source_breakdown} />
        <Breakdown title="By provider/model" values={funnel.provider_breakdown} />
      </Box>
      <Box>
        <Typography variant="h3" sx={{ mb: 1 }}>
          Rejections by reason
        </Typography>
        <Breakdown values={funnel.rejected_by_reason} empty="No hard-filter rejections." />
      </Box>
    </Paper>
  );
}

function Metric({ label, value }: { label: string; value: number | string }) {
  return (
    <Box sx={{ p: 1.5, borderRadius: 2, border: '1px solid', borderColor: 'divider' }}>
      <Typography variant="caption" color="text.secondary">
        {label}
      </Typography>
      <Typography variant="h3">{value}</Typography>
    </Box>
  );
}

function Breakdown({
  title,
  values,
  empty = 'No data.',
}: {
  title?: string;
  values: Record<string, number>;
  empty?: string;
}) {
  const entries = Object.entries(values);
  return (
    <Box sx={{ display: 'grid', gap: 0.75 }}>
      {title ? <Typography variant="h3">{title}</Typography> : null}
      {entries.length === 0 ? (
        <Typography variant="body2" color="text.secondary">
          {empty}
        </Typography>
      ) : (
        entries.map(([label, count]) => (
          <Box key={label} sx={{ display: 'flex', justifyContent: 'space-between', gap: 2 }}>
            <Typography variant="body2" color="text.secondary">
              {label}
            </Typography>
            <Typography variant="body2" sx={{ fontWeight: 700 }}>
              {count}
            </Typography>
          </Box>
        ))
      )}
    </Box>
  );
}
