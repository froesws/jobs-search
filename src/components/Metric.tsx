import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';

export function Metric({ label, value }: { label: string; value: string }) {
  return (
    <Box
      sx={{
        minWidth: 110,
        p: 1.5,
        borderRadius: 2,
        border: '1px solid',
        borderColor: 'divider',
        bgcolor: 'background.paper',
      }}
    >
      <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
        {label}
      </Typography>
      <Typography variant="h3">{value}</Typography>
    </Box>
  );
}
