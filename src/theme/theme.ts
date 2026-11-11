import { alpha, createTheme } from '@mui/material/styles';

import { tokens } from './tokens';

export type ThemeMode = 'dark' | 'light';

const accent = tokens.accent;
const accentSecondary = tokens.accentSecondary;

export function appTheme(mode: ThemeMode) {
  const dark = mode === 'dark';
  return createTheme({
    palette: {
      mode,
      primary: { main: accent },
      secondary: { main: accentSecondary },
      background: {
        default: dark ? '#0b0f19' : '#f4f6fb',
        paper: dark ? '#121827' : '#ffffff',
      },
      divider: dark ? alpha('#93a4c4', 0.14) : alpha('#1d2942', 0.12),
      text: {
        primary: dark ? '#edf1fb' : '#131a2a',
        secondary: dark ? '#9aa8c2' : '#51607a',
      },
      success: { main: dark ? '#5eead4' : '#0f9d80' },
      warning: { main: dark ? '#fbbf24' : '#b7791f' },
      error: { main: dark ? '#fb7185' : '#c53030' },
    },
    shape: { borderRadius: 14 },
    typography: {
      fontFamily: '"Inter", "system-ui", "Segoe UI", sans-serif',
      h1: { fontSize: '1.7rem', fontWeight: 700, letterSpacing: '-0.03em' },
      h2: { fontSize: '1.2rem', fontWeight: 700, letterSpacing: '-0.02em' },
      h3: { fontSize: '1rem', fontWeight: 650 },
      body1: { fontSize: '0.92rem' },
      body2: { fontSize: '0.82rem' },
      button: { textTransform: 'none', fontWeight: 600 },
    },
    components: {
      MuiCssBaseline: {
        styleOverrides: {
          body: {
            minWidth: 1280,
            minHeight: 800,
            backgroundImage: dark
              ? `radial-gradient(1100px 520px at 12% -8%, ${alpha(accent, 0.18)} 0%, transparent 60%),
                 radial-gradient(900px 420px at 92% 4%, ${alpha(accentSecondary, 0.1)} 0%, transparent 58%)`
              : `radial-gradient(1100px 520px at 12% -8%, ${alpha(accent, 0.12)} 0%, transparent 60%),
                 radial-gradient(900px 420px at 92% 4%, ${alpha(accentSecondary, 0.08)} 0%, transparent 58%)`,
            backgroundAttachment: 'fixed',
          },
          '::selection': { background: alpha(accent, 0.35) },
        },
      },
      MuiPaper: {
        styleOverrides: {
          root: {
            backgroundImage: 'none',
            border: `1px solid ${dark ? alpha('#93a4c4', 0.12) : alpha('#1d2942', 0.08)}`,
          },
        },
      },
      MuiCard: {
        styleOverrides: {
          root: {
            borderRadius: 18,
            backgroundColor: dark ? alpha('#151d30', 0.88) : '#ffffff',
            backdropFilter: 'blur(10px)',
          },
        },
      },
      MuiButton: {
        defaultProps: { disableElevation: true },
        styleOverrides: { root: { borderRadius: 10 } },
      },
      MuiChip: {
        styleOverrides: { root: { borderRadius: 8, fontWeight: 600 } },
      },
      MuiLinearProgress: {
        styleOverrides: { root: { borderRadius: 999, height: 6 } },
      },
    },
  });
}
