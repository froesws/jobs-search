import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';

interface ErrorStateProps {
  title?: string;
  message: string;
  onRetry?: () => void;
}

export function ErrorState({
  title = 'The local backend did not answer',
  message,
  onRetry,
}: ErrorStateProps) {
  return (
    <Box
      role="alert"
      sx={{
        minHeight: 360,
        display: 'grid',
        placeItems: 'center',
        px: 4,
        textAlign: 'center',
      }}
    >
      <Box
        sx={{
          maxWidth: 520,
          display: 'grid',
          justifyItems: 'center',
          gap: 2,
          p: 4,
          borderRadius: 4,
          border: '1px solid',
          borderColor: 'divider',
          bgcolor: 'background.paper',
        }}
      >
        <Box
          aria-hidden
          sx={{
            width: 54,
            height: 54,
            borderRadius: '50%',
            display: 'grid',
            placeItems: 'center',
            bgcolor: 'rgba(251, 113, 133, 0.12)',
            color: 'error.main',
            fontSize: 26,
            fontWeight: 700,
          }}
        >
          !
        </Box>
        <Typography variant="h2">{title}</Typography>
        <Typography color="text.secondary">{message}</Typography>
        {onRetry ? (
          <Button variant="contained" onClick={onRetry}>
            Try again
          </Button>
        ) : null}
      </Box>
    </Box>
  );
}
