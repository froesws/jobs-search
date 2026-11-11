import Alert from '@mui/material/Alert';
import AlertTitle from '@mui/material/AlertTitle';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import { Component } from 'react';
import type { ErrorInfo, ReactNode } from 'react';

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
}

/**
 * Catches render errors in the dashboard so a single failing component shows a
 * recoverable message instead of blanking the whole window.
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  override state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  override componentDidCatch(_error: Error, _info: ErrorInfo) {
    // React already reports the error to the console. Do not log the message
    // here: it may contain data from the failing component.
  }

  private handleRetry = () => {
    this.setState({ error: null });
  };

  override render() {
    const { error } = this.state;
    if (!error) return this.props.children;

    return (
      <Box sx={{ p: 4 }}>
        <Alert
          severity="error"
          action={
            <Button color="inherit" size="small" onClick={this.handleRetry}>
              Try again
            </Button>
          }
        >
          <AlertTitle>Something went wrong</AlertTitle>
          The interface hit an unexpected error. Try again, or restart the app if it keeps
          happening.
        </Alert>
      </Box>
    );
  }
}
