import Box from '@mui/material/Box';
import Paper from '@mui/material/Paper';
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableHead from '@mui/material/TableHead';
import TableRow from '@mui/material/TableRow';
import Typography from '@mui/material/Typography';

import type { LlmMetricsPayload } from '../lib/types';

export function LlmMetricsTable({ metrics }: { metrics: LlmMetricsPayload }) {
  return (
    <Paper sx={{ p: 2.5, display: 'grid', gap: 1.5 }}>
      <Typography variant="h2">LLM metrics</Typography>
      <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
        <Summary label="Calls" value={metrics.call_count} />
        <Summary label="Success" value={metrics.success_count} />
        <Summary label="Errors" value={metrics.error_count} />
        <Summary label="Fallbacks" value={metrics.fallback_count} />
        <Summary label="p50" value={`${metrics.p50_latency_ms} ms`} />
        <Summary label="p95" value={`${metrics.p95_latency_ms} ms`} />
      </Box>
      {metrics.by_provider.length > 0 ? (
        <Table size="small">
          <TableHead>
            <TableRow>
              <TableCell>Provider</TableCell>
              <TableCell>Model</TableCell>
              <TableCell align="right">Calls</TableCell>
              <TableCell align="right">Errors</TableCell>
              <TableCell align="right">Fallbacks</TableCell>
              <TableCell align="right">p95</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {metrics.by_provider.map((row) => (
              <TableRow key={`${row.provider}/${row.model}`}>
                <TableCell>{row.provider}</TableCell>
                <TableCell>{row.model}</TableCell>
                <TableCell align="right">{row.call_count}</TableCell>
                <TableCell align="right">{row.error_count}</TableCell>
                <TableCell align="right">{row.fallback_count}</TableCell>
                <TableCell align="right">{row.p95_latency_ms} ms</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : (
        <Typography variant="body2" color="text.secondary">
          No LLM calls in this scope.
        </Typography>
      )}
    </Paper>
  );
}

function Summary({ label, value }: { label: string; value: number | string }) {
  return (
    <Box sx={{ minWidth: 80 }}>
      <Typography variant="caption" color="text.secondary">
        {label}
      </Typography>
      <Typography variant="h3">{value}</Typography>
    </Box>
  );
}
