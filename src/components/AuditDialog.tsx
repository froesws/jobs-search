import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Divider from '@mui/material/Divider';
import Typography from '@mui/material/Typography';
import { useEffect, useState } from 'react';

import { safeMessage } from '../lib/errors';
import { getAudit } from '../lib/ipc';
import type { ExecutionAudit } from '../lib/types';
import { ErrorState } from './ErrorState';
import { Metric } from './Metric';
import { LoadingState } from './LoadingState';

export function AuditDialog({
  runId,
  open,
  onClose,
}: {
  runId: string | null;
  open: boolean;
  onClose: () => void;
}) {
  const [audit, setAudit] = useState<ExecutionAudit | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open || !runId) return;
    let active = true;
    setAudit(null);
    setError(null);
    getAudit(runId)
      .then((value) => {
        if (active) setAudit(value);
      })
      .catch((cause) => {
        if (active) setError(safeMessage(cause));
      });
    return () => {
      active = false;
    };
  }, [open, runId]);

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="md">
      <DialogTitle>Execution audit</DialogTitle>
      <DialogContent dividers>
        {!audit && !error ? <LoadingState label="Loading the run audit…" /> : null}
        {error ? <ErrorState message={error} /> : null}
        {audit ? (
          <Box sx={{ display: 'grid', gap: 2 }}>
            <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
              <Metric label="Status" value={audit.status} />
              <Metric label="Ingested" value={String(audit.total_ingested)} />
              <Metric label="Duplicates" value={String(audit.duplicates_merged)} />
              <Metric label="Rejected" value={String(audit.rejected_count)} />
              <Metric label="Pending" value={String(audit.pending_count)} />
              <Metric label="Approved" value={String(audit.approved_count)} />
              <Metric label="Errors" value={String(audit.error_count)} />
              <Metric label="Applied" value={String(audit.applied_count)} />
              <Metric label="Ignored" value={String(audit.ignored_count)} />
              <Metric label="Approval rate" value={`${audit.approval_rate.toFixed(1)}%`} />
              <Metric label="Application rate" value={`${audit.application_rate.toFixed(1)}%`} />
            </Box>
            <Divider />
            <Section title="Rejections by reason">
              {Object.keys(audit.rejected_by_reason).length === 0 ? (
                <Muted>No hard-filter rejections.</Muted>
              ) : (
                Object.entries(audit.rejected_by_reason).map(([reason, count]) => (
                  <Row key={reason} label={reason} value={String(count)} />
                ))
              )}
            </Section>
            <Section title="Source failures">
              {audit.source_failures.length === 0 ? (
                <Muted>No source failures.</Muted>
              ) : (
                <Alert severity="warning" variant="outlined">
                  {audit.source_failures.join(' · ')}
                </Alert>
              )}
            </Section>
            <Box sx={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 2 }}>
              <Section title="By source">
                {Object.entries(audit.source_breakdown).map(([source, count]) => (
                  <Row key={source} label={source} value={String(count)} />
                ))}
              </Section>
              <Section title="By provider/model">
                {Object.entries(audit.provider_breakdown).map(([provider, count]) => (
                  <Row key={provider} label={provider} value={String(count)} />
                ))}
              </Section>
            </Box>
            <Section title="Score distribution">
              <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap' }}>
                {Object.entries(audit.score_distribution).map(([bucket, count]) => (
                  <Metric key={bucket} label={bucket} value={String(count)} />
                ))}
              </Box>
            </Section>
          </Box>
        ) : null}
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose}>Close</Button>
      </DialogActions>
    </Dialog>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <Box sx={{ display: 'grid', gap: 0.75 }}>
      <Typography variant="h3">{title}</Typography>
      {children}
    </Box>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <Box sx={{ display: 'flex', justifyContent: 'space-between', gap: 2 }}>
      <Typography variant="body2" color="text.secondary">
        {label}
      </Typography>
      <Typography variant="body2" sx={{ fontWeight: 700 }}>
        {value}
      </Typography>
    </Box>
  );
}

function Muted({ children }: { children: React.ReactNode }) {
  return (
    <Typography variant="body2" color="text.secondary">
      {children}
    </Typography>
  );
}
