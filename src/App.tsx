import CssBaseline from '@mui/material/CssBaseline';
import { ThemeProvider } from '@mui/material/styles';
import { useEffect, useMemo, useState } from 'react';

import { ErrorBoundary } from '@/components/ErrorBoundary';
import { FeedbackProvider } from '@/components/FeedbackProvider';
import { appTheme } from '@/theme/theme';
import type { ThemeMode } from '@/theme/theme';
import { DashboardView } from '@/views/DashboardView';

const THEME_KEY = 'jobs-search.theme';

function initialMode(): ThemeMode {
  try {
    const saved = window.localStorage.getItem(THEME_KEY);
    if (saved === 'dark' || saved === 'light') return saved;
  } catch {
    // localStorage can be unavailable in hardened webviews.
  }
  return 'dark';
}

export default function App() {
  const [mode, setMode] = useState<ThemeMode>(initialMode);
  const theme = useMemo(() => appTheme(mode), [mode]);

  useEffect(() => {
    try {
      window.localStorage.setItem(THEME_KEY, mode);
    } catch {
      // The preference simply does not persist if storage is unavailable.
    }
    document.documentElement.style.colorScheme = mode;
  }, [mode]);

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <FeedbackProvider>
        <ErrorBoundary>
          <DashboardView
            mode={mode}
            onToggleMode={() => setMode((current) => (current === 'dark' ? 'light' : 'dark'))}
          />
        </ErrorBoundary>
      </FeedbackProvider>
    </ThemeProvider>
  );
}
