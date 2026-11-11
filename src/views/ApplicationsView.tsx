import Box from '@mui/material/Box';
import Chip from '@mui/material/Chip';
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableHead from '@mui/material/TableHead';
import TableRow from '@mui/material/TableRow';
import Typography from '@mui/material/Typography';

import { ratioLabel, statusLabel } from '../lib/format';
import type { JobSummary } from '../lib/types';
import { useApplicationsViewModel } from '../viewmodels/useApplicationsViewModel';
import { EmptyState } from '../components/EmptyState';
import { ErrorState } from '../components/ErrorState';
import { LoadingState } from '../components/LoadingState';
import { RequirementRatio } from '../components/RequirementRatio';
import { ScoreBadge } from '../components/ScoreBadge';

export function ApplicationsView({
  refreshToken,
  onOpenJob,
}: {
  refreshToken: number;
  onOpenJob: (job: JobSummary) => void;
}) {
  const jobs = useApplicationsViewModel(refreshToken);

  return (
    <Box sx={{ display: 'grid', gap: 2 }}>
      <Box>
        <Typography variant="h1">My Applications</Typography>
        <Typography color="text.secondary" variant="body2">
          Applied roles stay here with their score and requirement coverage.
        </Typography>
      </Box>

      {jobs.status === 'loading' ? <LoadingState label="Loading applications…" /> : null}
      {jobs.status === 'error' ? (
        <ErrorState message={jobs.error ?? 'Unknown backend error'} onRetry={jobs.retry} />
      ) : null}
      {jobs.status === 'empty' ? (
        <EmptyState
          title="No applications yet"
          description="Application tracking becomes available when the lifecycle backend lands. Until then, this tab stays intentionally empty."
        />
      ) : null}
      {jobs.status === 'ready' ? (
        <Box
          sx={{
            border: '1px solid',
            borderColor: 'divider',
            borderRadius: 3,
            overflow: 'hidden',
            bgcolor: 'background.paper',
          }}
        >
          <Table>
            <TableHead>
              <TableRow>
                <TableCell>Role</TableCell>
                <TableCell>Company</TableCell>
                <TableCell>Status</TableCell>
                <TableCell>Requirements</TableCell>
                <TableCell align="right">Score</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {jobs.items.map((job) => (
                <TableRow
                  key={job.job_id}
                  hover
                  onClick={() => onOpenJob(job)}
                  sx={{ cursor: 'pointer' }}
                >
                  <TableCell>
                    <Typography variant="body2" sx={{ fontWeight: 600 }}>
                      {job.title}
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
                      {job.location}
                    </Typography>
                  </TableCell>
                  <TableCell>{job.company}</TableCell>
                  <TableCell>
                    <Chip
                      size="small"
                      label={statusLabel(job.evaluation_status)}
                      variant="outlined"
                    />
                  </TableCell>
                  <TableCell>
                    <Typography variant="caption">{ratioLabel(job)}</Typography>
                    <RequirementRatio job={job} compact />
                  </TableCell>
                  <TableCell align="right">
                    <Box sx={{ display: 'flex', justifyContent: 'flex-end' }}>
                      <ScoreBadge score={job.match_score} size={44} />
                    </Box>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Box>
      ) : null}
    </Box>
  );
}
