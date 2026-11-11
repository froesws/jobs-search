import Box from '@mui/material/Box';
import Chip from '@mui/material/Chip';
import Paper from '@mui/material/Paper';
import Typography from '@mui/material/Typography';

export function AnalyticsView() {
  return (
    <Box sx={{ display: 'grid', gap: 2 }}>
      <Box>
        <Typography variant="h1">Analytics</Typography>
        <Typography color="text.secondary" variant="body2">
          Market gaps and pipeline health, computed locally from your evaluations.
        </Typography>
      </Box>

      <Paper
        sx={{
          p: 4,
          display: 'grid',
          gap: 2,
          background:
            'linear-gradient(135deg, rgba(124,140,255,0.20) 0%, rgba(94,234,212,0.08) 55%, rgba(18,24,39,0.6) 100%)',
        }}
      >
        <Chip
          label="Coming in 003"
          color="secondary"
          variant="outlined"
          sx={{ justifySelf: 'start' }}
        />
        <Typography variant="h1" sx={{ maxWidth: 720 }}>
          Turn evaluated jobs into a learning roadmap and a funnel you can trust.
        </Typography>
        <Typography color="text.secondary" sx={{ maxWidth: 720 }}>
          Skill Gap ranks the technologies your target market asks for but your profile does not
          cover. Funnel shows where opportunities are won or lost, by source and provider.
        </Typography>
        <Box sx={{ display: 'grid', gridTemplateColumns: 'repeat(3, minmax(0, 1fr))', gap: 2 }}>
          <Preview title="Skill Gap" text="Demand count, percentage, and ranked progress bars." />
          <Preview
            title="Funnel"
            text="Ingested, duplicates, rejections, approvals, and applications."
          />
          <Preview title="CSV export" text="Current view, local file, sensitive-data warning." />
        </Box>
      </Paper>
    </Box>
  );
}

function Preview({ title, text }: { title: string; text: string }) {
  return (
    <Box
      sx={{
        p: 2,
        borderRadius: 3,
        border: '1px dashed',
        borderColor: 'divider',
        bgcolor: 'rgba(11,15,25,0.35)',
        display: 'grid',
        gap: 0.5,
      }}
    >
      <Typography variant="h3">{title}</Typography>
      <Typography variant="caption" color="text.secondary">
        {text}
      </Typography>
    </Box>
  );
}
