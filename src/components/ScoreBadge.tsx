import Box from '@mui/material/Box';
import Typography from '@mui/material/Typography';
import { useTheme } from '@mui/material/styles';

import { scoreLabel } from '@/lib/format';
import { scoreTone } from '@/theme/tokens';

export function ScoreBadge({
  score,
  size = 58,
}: {
  score: number | null | undefined;
  size?: number;
}) {
  const theme = useTheme();
  const tone = scoreTone(score);
  const color =
    tone === 'success'
      ? theme.palette.success.main
      : tone === 'warning'
        ? theme.palette.warning.main
        : tone === 'error'
          ? theme.palette.error.main
          : theme.palette.text.disabled;
  const value = scoreLabel(score);

  return (
    <Box
      aria-label={score === null || score === undefined ? 'Not scored' : `Match score ${value}`}
      sx={{
        width: size,
        height: size,
        flex: '0 0 auto',
        borderRadius: '50%',
        display: 'grid',
        placeItems: 'center',
        background: `conic-gradient(${color} ${Math.max(0, Math.min(100, Number(value) || 0)) * 3.6}deg, ${theme.palette.action.hover} 0deg)`,
        p: '4px',
      }}
    >
      <Box
        sx={{
          width: '100%',
          height: '100%',
          borderRadius: '50%',
          bgcolor: 'background.paper',
          display: 'grid',
          placeItems: 'center',
          border: '1px solid',
          borderColor: 'divider',
        }}
      >
        <Typography sx={{ fontSize: size * 0.3, fontWeight: 750, lineHeight: 1, color }}>
          {value}
        </Typography>
      </Box>
    </Box>
  );
}
