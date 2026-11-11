import Box from '@mui/material/Box';
import CircularProgress from '@mui/material/CircularProgress';
import Typography from '@mui/material/Typography';

export function LoadingState({ label = 'Loading your workspace…' }: { label?: string }) {
  return (
    <Box
      role="status"
      aria-live="polite"
      sx={{
        minHeight: 420,
        display: 'grid',
        placeItems: 'center',
        gap: 2,
        textAlign: 'center',
      }}
    >
      <Box sx={{ display: 'grid', justifyItems: 'center', gap: 2 }}>
        <Box sx={{ position: 'relative', display: 'grid', placeItems: 'center' }}>
          <CircularProgress size={54} thickness={2.4} />
          <Box
            sx={{
              position: 'absolute',
              width: 10,
              height: 10,
              borderRadius: '50%',
              bgcolor: 'secondary.main',
              boxShadow: '0 0 18px rgba(94, 234, 212, 0.8)',
            }}
          />
        </Box>
        <Typography color="text.secondary" variant="body2">
          {label}
        </Typography>
      </Box>
    </Box>
  );
}
