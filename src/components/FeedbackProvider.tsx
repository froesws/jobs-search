import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import Snackbar from '@mui/material/Snackbar';
import { createContext, useCallback, useContext, useMemo, useState } from 'react';
import type { ReactNode } from 'react';

type Severity = 'success' | 'info' | 'warning' | 'error';

interface FeedbackItem {
  id: number;
  message: string;
  severity: Severity;
  persistent: boolean;
}

interface FeedbackApi {
  notify: (message: string, severity?: Severity, persistent?: boolean) => void;
}

const FeedbackContext = createContext<FeedbackApi | null>(null);

export function FeedbackProvider({ children }: { children: ReactNode }) {
  const [queue, setQueue] = useState<FeedbackItem[]>([]);

  const dismiss = useCallback(() => {
    setQueue((current) => current.slice(1));
  }, []);

  const notify = useCallback((message: string, severity: Severity = 'info', persistent = false) => {
    setQueue((current) => [
      ...current,
      { id: Date.now() + current.length, message, severity, persistent },
    ]);
  }, []);

  const api = useMemo(() => ({ notify }), [notify]);
  const current = queue[0];

  return (
    <FeedbackContext.Provider value={api}>
      {children}
      <Snackbar
        key={current?.id ?? 'empty'}
        open={Boolean(current)}
        autoHideDuration={current?.persistent ? null : 4200}
        onClose={dismiss}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
      >
        {current ? (
          <Alert
            severity={current.severity}
            variant="filled"
            onClose={dismiss}
            action={
              current.persistent ? (
                <Button color="inherit" size="small" onClick={dismiss}>
                  Dismiss
                </Button>
              ) : undefined
            }
            sx={{ minWidth: 320, boxShadow: 6 }}
          >
            {current.message}
          </Alert>
        ) : undefined}
      </Snackbar>
    </FeedbackContext.Provider>
  );
}

export function useFeedback(): FeedbackApi {
  const value = useContext(FeedbackContext);
  if (!value) {
    throw new Error('useFeedback must be used inside FeedbackProvider');
  }
  return value;
}
