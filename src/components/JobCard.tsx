import Box from '@mui/material/Box';
import Card from '@mui/material/Card';
import CardActionArea from '@mui/material/CardActionArea';
import Chip from '@mui/material/Chip';
import Typography from '@mui/material/Typography';

import { modalityLabel, postingAge, statusLabel } from '../lib/format';
import type { JobSummary } from '../lib/types';
import { RequirementRatio } from './RequirementRatio';
import { ScoreBadge } from './ScoreBadge';

export function JobCard({ job, onOpen }: { job: JobSummary; onOpen: (jobId: string) => void }) {
  const isBelowThreshold = job.evaluation_status === 'PENDING';

  return (
    <Card
      sx={{
        height: '100%',
        transition: 'transform 160ms ease, border-color 160ms ease, box-shadow 160ms ease',
        '&:hover': {
          transform: 'translateY(-2px)',
          borderColor: 'primary.main',
          boxShadow: '0 18px 45px rgba(0, 0, 0, 0.28)',
        },
      }}
    >
      <CardActionArea
        onClick={() => onOpen(job.job_id)}
        sx={{ height: '100%', p: 2.5, alignItems: 'stretch' }}
      >
        <Box sx={{ display: 'flex', gap: 2, height: '100%', flexDirection: 'column' }}>
          <Box sx={{ display: 'flex', gap: 2, alignItems: 'flex-start' }}>
            <Box sx={{ flex: 1, minWidth: 0 }}>
              <Typography
                variant="h3"
                sx={{
                  display: '-webkit-box',
                  WebkitLineClamp: 2,
                  WebkitBoxOrient: 'vertical',
                  overflow: 'hidden',
                  mb: 0.5,
                }}
              >
                {job.title}
              </Typography>
              <Typography color="text.secondary" variant="body2" noWrap>
                {job.company}
              </Typography>
            </Box>
            <ScoreBadge score={job.match_score} />
          </Box>

          <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.75 }}>
            <Chip size="small" label={job.location} variant="outlined" />
            <Chip
              size="small"
              label={modalityLabel(job.work_modality)}
              color={job.work_modality === 'Remote' ? 'secondary' : 'default'}
              variant="outlined"
            />
            <Chip
              size="small"
              label={isBelowThreshold ? 'Below threshold' : statusLabel(job.evaluation_status)}
              color={isBelowThreshold ? 'warning' : 'success'}
              variant="outlined"
            />
            {job.needs_review ? (
              <Chip size="small" label="Needs review" color="error" variant="outlined" />
            ) : null}
          </Box>

          <Typography
            color="text.secondary"
            variant="body2"
            sx={{
              flex: 1,
              display: '-webkit-box',
              WebkitLineClamp: 3,
              WebkitBoxOrient: 'vertical',
              overflow: 'hidden',
            }}
          >
            {job.summary || 'No summary available for this posting.'}
          </Typography>

          <RequirementRatio job={job} compact />

          <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Typography variant="caption" color="text.secondary">
              {postingAge(job.posted_at)}
            </Typography>
            <Typography variant="caption" color="primary.main" sx={{ fontWeight: 700 }}>
              Open details →
            </Typography>
          </Box>
        </Box>
      </CardActionArea>
    </Card>
  );
}
