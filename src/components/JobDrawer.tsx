import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Divider from '@mui/material/Divider';
import Drawer from '@mui/material/Drawer';
import Typography from '@mui/material/Typography';

import { formatDate, modalityLabel, postingAge, safeExternalUrl, statusLabel } from '@/lib/format';
import { openExternalUrl } from '@/lib/ipc';
import type { JobDetail } from '@/lib/types';
import { tokens } from '@/theme/tokens';
import { ErrorState } from '@/components/ErrorState';
import { useFeedback } from '@/components/FeedbackProvider';
import { RequirementRatio } from '@/components/RequirementRatio';
import { ScoreBadge } from '@/components/ScoreBadge';

const DIMENSIONS = [
  ['CORE_TECHNICAL', 'Core technical'],
  ['DOMAIN', 'Domain & architecture'],
  ['SENIORITY', 'Seniority & role'],
  ['SECONDARY', 'Secondary'],
] as const;

export function JobDrawer({
  open,
  job,
  status,
  error,
  busy = false,
  onClose,
  onRetry,
  onApply,
  onIgnore,
  onRestore,
  onApprove,
}: {
  open: boolean;
  job: JobDetail | null;
  status: 'idle' | 'loading' | 'ready' | 'error';
  error: string | null;
  busy?: boolean;
  onClose: () => void;
  onRetry: () => void;
  onApply?: (jobId: string) => void;
  onIgnore?: (jobId: string) => void;
  onRestore?: (jobId: string) => void;
  onApprove?: (jobId: string) => void;
}) {
  const url = job ? safeExternalUrl(job.url) : null;
  const feedback = useFeedback();

  return (
    <Drawer
      anchor="right"
      open={open}
      onClose={onClose}
      slotProps={{
        paper: {
          sx: {
            width: tokens.drawerWidth,
            maxWidth: '100vw',
            bgcolor: 'background.default',
            backgroundImage:
              'radial-gradient(700px 320px at 100% 0%, rgba(124,140,255,0.14), transparent 65%)',
          },
        },
      }}
    >
      <Box sx={{ p: 3, display: 'flex', flexDirection: 'column', gap: 2.5, height: '100%' }}>
        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <Typography variant="overline" color="text.secondary">
            Job detail
          </Typography>
          <Button size="small" onClick={onClose} aria-label="Close job detail">
            Close
          </Button>
        </Box>

        {status === 'loading' ? (
          <Box sx={{ py: 6, display: 'grid', placeItems: 'center' }}>
            <Typography color="text.secondary">Loading job detail…</Typography>
          </Box>
        ) : null}

        {status === 'error' ? (
          <ErrorState
            title="Could not load this job"
            message={error ?? 'Unknown error'}
            onRetry={onRetry}
          />
        ) : null}

        {job && status === 'ready' ? (
          <>
            <Box sx={{ display: 'flex', gap: 2, alignItems: 'flex-start' }}>
              <Box sx={{ flex: 1 }}>
                <Typography variant="h1" sx={{ fontSize: '1.45rem', mb: 0.75 }}>
                  {job.title}
                </Typography>
                <Typography color="text.secondary">{job.company}</Typography>
              </Box>
              <ScoreBadge score={job.match_score} size={72} />
            </Box>

            <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.75 }}>
              <Chip size="small" label={job.location} variant="outlined" />
              <Chip size="small" label={modalityLabel(job.work_modality)} variant="outlined" />
              <Chip
                size="small"
                label={statusLabel(job.evaluation_status)}
                color="success"
                variant="outlined"
              />
              <Chip size="small" label={postingAge(job.posted_at)} variant="outlined" />
            </Box>

            <Box sx={{ display: 'flex', gap: 2, alignItems: 'center' }}>
              <Box sx={{ flex: 1 }}>
                <RequirementRatio job={job} />
              </Box>
              <Typography variant="caption" color="text.secondary">
                {formatDate(job.posted_at)}
              </Typography>
            </Box>

            {job.hard_filter_rejections.length > 0 ? (
              <Alert severity="error" variant="outlined">
                {job.hard_filter_rejections.map((reason) => reason.message).join(' · ')}
              </Alert>
            ) : null}

            <Divider />

            <Section title="Description">
              <Typography
                variant="body2"
                color="text.secondary"
                sx={{ whiteSpace: 'pre-wrap', lineHeight: 1.65 }}
              >
                {job.description_clean || 'No clean description was stored for this posting.'}
              </Typography>
            </Section>

            <Section title="Requirements">
              {job.requirements.length === 0 ? (
                <Typography variant="body2" color="text.secondary">
                  No requirements were extracted. This is shown as “No requirements”, not a perfect
                  match.
                </Typography>
              ) : (
                <Box sx={{ display: 'grid', gap: 1.5 }}>
                  {DIMENSIONS.map(([key, label]) => {
                    const items = job.requirements.filter((item) => item.dimension === key);
                    if (items.length === 0) return null;
                    return (
                      <Box key={key}>
                        <Typography
                          variant="caption"
                          color="text.secondary"
                          sx={{ fontWeight: 700 }}
                        >
                          {label}
                        </Typography>
                        <Box sx={{ display: 'grid', gap: 0.5, mt: 0.5 }}>
                          {items.map((item) => (
                            <Box
                              key={item.id}
                              sx={{ display: 'flex', gap: 1, alignItems: 'center' }}
                            >
                              <Chip
                                size="small"
                                label={
                                  item.match_type === 'EXACT'
                                    ? 'Exact'
                                    : item.match_type === 'INFERRED'
                                      ? 'Inferred'
                                      : 'Missing'
                                }
                                color={
                                  item.match_type === 'NONE'
                                    ? 'error'
                                    : item.match_type === 'INFERRED'
                                      ? 'warning'
                                      : 'success'
                                }
                                variant="outlined"
                              />
                              <Typography variant="body2">{item.text}</Typography>
                            </Box>
                          ))}
                        </Box>
                      </Box>
                    );
                  })}
                </Box>
              )}
            </Section>

            <Box sx={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 2 }}>
              <Section title="Matched">
                <Chips values={job.matched_keywords} color="success" empty="None yet" />
              </Section>
              <Section title="Missing">
                <Chips values={job.missing_keywords} color="error" empty="None" />
              </Section>
            </Box>

            <Section title="Source lineage">
              <Box sx={{ display: 'grid', gap: 0.75 }}>
                {job.source_lineage.map((lineage) => (
                  <Box
                    key={`${lineage.source_id}-${lineage.external_id}`}
                    sx={{ display: 'flex', justifyContent: 'space-between', gap: 2 }}
                  >
                    <Typography variant="body2">{lineage.source_name}</Typography>
                    <Typography variant="caption" color="text.secondary">
                      {formatDate(lineage.ingested_at)}
                    </Typography>
                  </Box>
                ))}
              </Box>
            </Section>

            <Box sx={{ mt: 'auto', pt: 2, display: 'grid', gap: 1 }}>
              {job.lifecycle_state === 'APPLIED' ? (
                <Alert severity="success" variant="outlined">
                  Applied on {formatDate(job.applied_at)}. The evaluation snapshot is preserved.
                </Alert>
              ) : null}
              {job.lifecycle_state === 'IGNORED' ? (
                <Button variant="outlined" onClick={() => onRestore?.(job.job_id)} disabled={busy}>
                  Restore to New Jobs
                </Button>
              ) : null}
              {job.evaluation_status === 'APPROVED' && job.lifecycle_state === 'DISCOVERED' ? (
                <Box sx={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 1 }}>
                  <Button variant="contained" onClick={() => onApply?.(job.job_id)} disabled={busy}>
                    I Applied
                  </Button>
                  <Button variant="outlined" onClick={() => onIgnore?.(job.job_id)} disabled={busy}>
                    Ignore
                  </Button>
                </Box>
              ) : null}
              {job.evaluation_status === 'PENDING' && job.match_score !== null ? (
                <Button
                  variant="outlined"
                  color="warning"
                  onClick={() => onApprove?.(job.job_id)}
                  disabled={busy}
                >
                  Approve manually (score {job.match_score})
                </Button>
              ) : null}
              {url ? (
                <Box>
                  <Typography variant="overline" color="text.secondary">
                    Original posting
                  </Typography>
                  <Typography
                    component="a"
                    href={url}
                    rel="noreferrer"
                    variant="body2"
                    data-testid="original-posting-url"
                    onClick={(event) => {
                      event.preventDefault();
                      void openExternalUrl(url).catch(() =>
                        feedback.notify(
                          `Could not open the posting link. Select the address in the drawer to copy it: ${url}`,
                          'error',
                        ),
                      );
                    }}
                    sx={{ display: 'block', color: 'primary.main' }}
                    style={{ wordBreak: 'break-all', userSelect: 'text' }}
                  >
                    {url}
                  </Typography>
                </Box>
              ) : (
                <Alert severity="warning" variant="outlined">
                  The original URL is missing or invalid. The stored job data is unchanged.
                </Alert>
              )}
              {job.provider ? (
                <Typography variant="caption" color="text.secondary" sx={{ textAlign: 'center' }}>
                  Evaluated with {job.provider}
                  {job.model ? ` / ${job.model}` : ''}
                </Typography>
              ) : null}
            </Box>
          </>
        ) : null}
      </Box>
    </Drawer>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <Box>
      <Typography variant="h3" sx={{ mb: 1 }}>
        {title}
      </Typography>
      {children}
    </Box>
  );
}

function Chips({
  values,
  color,
  empty,
}: {
  values: string[];
  color: 'success' | 'error';
  empty: string;
}) {
  if (values.length === 0) {
    return (
      <Typography variant="body2" color="text.secondary">
        {empty}
      </Typography>
    );
  }
  return (
    <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
      {values.map((value) => (
        <Chip key={value} size="small" label={value} color={color} variant="outlined" />
      ))}
    </Box>
  );
}
