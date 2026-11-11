import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Typography from '@mui/material/Typography';
import type { ReactNode } from 'react';

interface EmptyStateProps {
  title: string;
  description: string;
  action?: ReactNode;
}

export function EmptyState({ title, description, action }: EmptyStateProps) {
  return (
    <Box
      sx={{
        minHeight: 380,
        display: 'grid',
        placeItems: 'center',
        textAlign: 'center',
        px: 4,
      }}
    >
      <Box sx={{ maxWidth: 460, display: 'grid', justifyItems: 'center', gap: 2 }}>
        <Box
          aria-hidden
          sx={{
            width: 84,
            height: 84,
            borderRadius: '50%',
            display: 'grid',
            placeItems: 'center',
            background:
              'radial-gradient(circle at 35% 30%, rgba(124,140,255,0.35), rgba(124,140,255,0.05) 70%)',
            border: '1px solid rgba(124,140,255,0.3)',
          }}
        >
          <Box
            sx={{
              width: 30,
              height: 30,
              borderRadius: '8px',
              border: '2px solid',
              borderColor: 'primary.main',
              transform: 'rotate(45deg)',
            }}
          />
        </Box>
        <Typography variant="h2">{title}</Typography>
        <Typography color="text.secondary">{description}</Typography>
        {action}
      </Box>
    </Box>
  );
}

export function EmptyStateAction({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <Button variant="outlined" onClick={onClick} sx={{ mt: 1 }}>
      {label}
    </Button>
  );
}
