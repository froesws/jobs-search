import Accordion from '@mui/material/Accordion';
import AccordionDetails from '@mui/material/AccordionDetails';
import AccordionSummary from '@mui/material/AccordionSummary';
import Alert from '@mui/material/Alert';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import Stack from '@mui/material/Stack';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useEffect, useMemo, useState } from 'react';

import { toAppError } from '@/lib/errors';
import type { ProviderId, ProviderSettingView } from '@/lib/types';
import type { ProviderConfigState } from '@/viewmodels/useProviderConfig';

const CREDENTIAL_STORE_UNAVAILABLE = 'CREDENTIAL_STORE_UNAVAILABLE';

export function ProviderConfigDialog({
  open,
  onClose,
  state,
}: {
  open: boolean;
  onClose: () => void;
  state: ProviderConfigState;
}) {
  const [apiKeys, setApiKeys] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [consentProvider, setConsentProvider] = useState<ProviderId | null>(null);
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});

  const providers = useMemo(() => state.config?.providers ?? [], [state.config]);

  useEffect(() => {
    if (!open) return;
    setApiKeys({});
    setError(null);
    setConsentProvider(null);
    setExpanded(
      Object.fromEntries(providers.map((provider) => [provider.id, provider.hasCredential])),
    );
  }, [open, providers]);

  const handleSaveKey = async (provider: ProviderSettingView, allowEnvFallback = false) => {
    const apiKey = apiKeys[provider.id]?.trim() ?? '';
    if (!apiKey) {
      setError('Enter an API key before saving.');
      return;
    }
    setBusy(provider.id);
    setError(null);
    try {
      await state.saveCredential(provider.id, apiKey, allowEnvFallback);
      setApiKeys((current) => ({ ...current, [provider.id]: '' }));
      setConsentProvider(null);
    } catch (cause) {
      const appError = toAppError(cause);
      if (appError.code === CREDENTIAL_STORE_UNAVAILABLE && !allowEnvFallback) {
        setConsentProvider(provider.id);
      } else {
        setError(appError.message);
      }
    } finally {
      setBusy(null);
    }
  };

  const handleRemove = async (provider: ProviderSettingView, removeEnvEntry: boolean) => {
    setBusy(provider.id);
    setError(null);
    try {
      await state.removeCredential(provider.id, removeEnvEntry);
    } catch (cause) {
      setError(toAppError(cause).message);
    } finally {
      setBusy(null);
    }
  };

  const handleReset = async (provider: ProviderSettingView, field: 'model' | 'attempts') => {
    setBusy(provider.id);
    setError(null);
    try {
      await state.resetSetting(provider.id, field);
    } catch (cause) {
      setError(toAppError(cause).message);
    } finally {
      setBusy(null);
    }
  };

  return (
    <>
      <Dialog open={open} onClose={onClose} fullWidth maxWidth="md">
        <DialogTitle>Provider settings</DialogTitle>
        <DialogContent dividers>
          {state.error ? (
            <Alert severity="error" sx={{ mb: 2 }}>
              {state.error}
            </Alert>
          ) : null}
          {error ? (
            <Alert severity="error" sx={{ mb: 2 }}>
              {error}
            </Alert>
          ) : null}

          <Stack spacing={1.5}>
            {providers.map((provider) => (
              <Accordion
                key={provider.id}
                expanded={expanded[provider.id] ?? false}
                onChange={(_, isExpanded) =>
                  setExpanded((current) => ({ ...current, [provider.id]: isExpanded }))
                }
                disableGutters
              >
                <AccordionSummary expandIcon={<span aria-hidden>▾</span>}>
                  <Stack direction="row" spacing={1} sx={{ width: '100%', alignItems: 'center' }}>
                    <Typography sx={{ flex: 1 }}>{provider.label}</Typography>
                    <Chip
                      size="small"
                      variant="outlined"
                      color={provider.hasCredential ? 'success' : 'default'}
                      label={
                        provider.hasCredential
                          ? `configured (${provider.credentialSource})`
                          : 'not configured'
                      }
                    />
                  </Stack>
                </AccordionSummary>
                <AccordionDetails>
                  <Stack spacing={1.5}>
                    {provider.requiresApiKey ? (
                      <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1}>
                        <TextField
                          size="small"
                          type="password"
                          label={`${provider.label} API key`}
                          value={apiKeys[provider.id] ?? ''}
                          onChange={(event) =>
                            setApiKeys((current) => ({
                              ...current,
                              [provider.id]: event.target.value,
                            }))
                          }
                          sx={{ flex: 1 }}
                          autoComplete="off"
                        />
                        <Button
                          variant="contained"
                          size="small"
                          disabled={busy === provider.id}
                          onClick={() => void handleSaveKey(provider)}
                        >
                          Save key
                        </Button>
                        <Button
                          variant="outlined"
                          size="small"
                          disabled={busy === provider.id || !provider.hasCredential}
                          onClick={() => void handleRemove(provider, false)}
                        >
                          Remove credential
                        </Button>
                      </Stack>
                    ) : (
                      <Typography variant="body2" color="text.secondary">
                        Ollama runs locally and does not use an API key.
                      </Typography>
                    )}

                    <Stack direction="row" spacing={1}>
                      <Button
                        size="small"
                        disabled={busy === provider.id}
                        onClick={() => void handleReset(provider, 'model')}
                      >
                        Reset model
                      </Button>
                      <Button
                        size="small"
                        disabled={busy === provider.id}
                        onClick={() => void handleReset(provider, 'attempts')}
                      >
                        Reset attempts
                      </Button>
                    </Stack>
                  </Stack>
                </AccordionDetails>
              </Accordion>
            ))}
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={onClose}>Close</Button>
        </DialogActions>
      </Dialog>

      <Dialog open={consentProvider !== null} onClose={() => setConsentProvider(null)}>
        <DialogTitle>Credential store unavailable</DialogTitle>
        <DialogContent>
          <Typography variant="body2">
            The OS credential store is not reachable. Store the key in the local <code>.env</code>{' '}
            file as plaintext with owner-only permissions instead?
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConsentProvider(null)}>Cancel</Button>
          <Button
            variant="contained"
            onClick={() => {
              const provider = providers.find((item) => item.id === consentProvider);
              if (provider) void handleSaveKey(provider, true);
            }}
          >
            Write to .env
          </Button>
        </DialogActions>
      </Dialog>
    </>
  );
}
