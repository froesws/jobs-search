import Accordion from '@mui/material/Accordion';
import AccordionDetails from '@mui/material/AccordionDetails';
import AccordionSummary from '@mui/material/AccordionSummary';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import DialogTitle from '@mui/material/DialogTitle';
import FormControl from '@mui/material/FormControl';
import FormControlLabel from '@mui/material/FormControlLabel';
import InputLabel from '@mui/material/InputLabel';
import MenuItem from '@mui/material/MenuItem';
import Select from '@mui/material/Select';
import Stack from '@mui/material/Stack';
import Switch from '@mui/material/Switch';
import TextField from '@mui/material/TextField';
import Typography from '@mui/material/Typography';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { PointerEvent as ReactPointerEvent } from 'react';

import { toAppError } from '@/lib/errors';
import { refreshProviderModels, testProvider } from '@/lib/ipc';
import type {
  ModelCatalogView,
  ProviderConfigInput,
  ProviderId,
  ProviderSettingView,
} from '@/lib/types';
import type { ProviderConfigState } from '@/viewmodels/useProviderConfig';

const CREDENTIAL_STORE_UNAVAILABLE = 'CREDENTIAL_STORE_UNAVAILABLE';

interface ProviderDraft {
  model: string;
  models: string[];
  baseUrl: string;
  attempts: number;
  enabled: boolean;
}

interface Draft {
  order: ProviderId[];
  fallbackEnabled: boolean;
  providers: Record<string, ProviderDraft>;
}

function emptyDraft(provider: ProviderSettingView): ProviderDraft {
  return {
    model: provider.model ?? '',
    models: provider.models ?? [],
    baseUrl: provider.baseUrl ?? '',
    attempts: provider.attempts,
    enabled: provider.enabled,
  };
}

function draftFrom(config: ProviderConfigState['config']): Draft {
  const providers: Record<string, ProviderDraft> = {};
  for (const provider of config?.providers ?? []) {
    providers[provider.id] = {
      model: provider.model ?? '',
      models: provider.models ?? [],
      baseUrl: provider.baseUrl ?? '',
      attempts: provider.attempts,
      enabled: provider.enabled,
    };
  }
  return {
    order: config?.order ?? [],
    fallbackEnabled: config?.fallbackEnabled ?? false,
    providers,
  };
}

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
  const [notice, setNotice] = useState<string | null>(null);
  const [consentProvider, setConsentProvider] = useState<ProviderId | null>(null);
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [draft, setDraft] = useState<Draft>(() => draftFrom(state.config));
  const [catalogs, setCatalogs] = useState<Record<string, ModelCatalogView>>({});
  const [showAll, setShowAll] = useState<Record<string, boolean>>({});
  const [dragging, setDragging] = useState<number | null>(null);
  const orderRows = useRef<Array<HTMLDivElement | null>>([]);
  const dragGhost = useRef<HTMLDivElement | null>(null);

  const providers = useMemo(() => state.config?.providers ?? [], [state.config]);

  const loadCatalog = useCallback(async (provider: ProviderId, includePaid: boolean) => {
    const catalog = await refreshProviderModels(provider, includePaid, false);
    setCatalogs((current) => ({ ...current, [provider]: catalog }));
  }, []);

  useEffect(() => {
    if (!open) return;
    setApiKeys({});
    setError(null);
    setNotice(null);
    setConsentProvider(null);
    setDraft(draftFrom(state.config));
    setExpanded(
      Object.fromEntries(providers.map((provider) => [provider.id, provider.hasCredential])),
    );
    for (const provider of providers) {
      void loadCatalog(provider.id, false).catch(() => undefined);
    }
  }, [open, providers, state.config, loadCatalog]);

  // The configuration loads asynchronously, so the draft must follow it even
  // while the dialog is closed. Without this the first render after the config
  // arrives would read a provider that is not in the draft yet.
  useEffect(() => {
    if (open) return;
    setDraft(draftFrom(state.config));
  }, [open, state.config]);

  // Ollama has no static catalog: query the local server when the dialog opens
  // and pre-select the first model it reports.
  useEffect(() => {
    if (!open) return;
    if (!providers.some((provider) => provider.id === 'ollama')) return;
    let cancelled = false;
    refreshProviderModels('ollama', false, true)
      .then((catalog) => {
        if (cancelled) return;
        setCatalogs((current) => ({ ...current, ollama: catalog }));
        const first = catalog.models[0]?.id;
        if (!first) return;
        setDraft((current) => {
          const value = current.providers.ollama;
          if (!value || value.model) return current;
          return {
            ...current,
            providers: { ...current.providers, ollama: { ...value, model: first } },
          };
        });
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [open, providers]);

  const labelFor = (providerId: ProviderId) =>
    state.config?.providers.find((provider) => provider.id === providerId)?.label ?? providerId;

  const startDrag = (event: ReactPointerEvent<HTMLElement>, index: number) => {
    event.preventDefault();
    event.currentTarget.setPointerCapture?.(event.pointerId);
    if (dragGhost.current) {
      dragGhost.current.style.transform = `translate(${event.clientX + 14}px, ${
        event.clientY - 18
      }px)`;
    }
    setDragging(index);
  };

  const moveProvider = (from: number, to: number) => {
    if (from === to) return;
    setDraft((current) => {
      if (to < 0 || to >= current.order.length) return current;
      const order = [...current.order];
      const [entry] = order.splice(from, 1);
      if (!entry) return current;
      order.splice(to, 0, entry);
      return { ...current, order };
    });
  };

  // Pointer-based dragging: WebKitGTK (the Tauri Linux webview) does not deliver
  // HTML5 drop events reliably, so the reorder is driven by pointer movement.
  useEffect(() => {
    if (dragging === null) return;

    const targetIndex = (clientY: number) => {
      for (let index = 0; index < orderRows.current.length; index += 1) {
        const element = orderRows.current[index];
        if (!element) continue;
        const rect = element.getBoundingClientRect();
        if (clientY >= rect.top && clientY <= rect.bottom) return index;
      }
      return null;
    };

    const handleMove = (event: PointerEvent) => {
      if (dragGhost.current) {
        dragGhost.current.style.transform = `translate(${event.clientX + 14}px, ${
          event.clientY - 18
        }px)`;
      }
      const target = targetIndex(event.clientY);
      if (target === null || target === dragging) return;
      moveProvider(dragging, target);
      setDragging(target);
    };
    const handleUp = () => setDragging(null);

    window.addEventListener('pointermove', handleMove);
    window.addEventListener('pointerup', handleUp);
    window.addEventListener('pointercancel', handleUp);
    return () => {
      window.removeEventListener('pointermove', handleMove);
      window.removeEventListener('pointerup', handleUp);
      window.removeEventListener('pointercancel', handleUp);
    };
  }, [dragging]);

  const updateProvider = (id: string, patch: Partial<ProviderDraft>) => {
    setDraft((current) => ({
      ...current,
      providers: {
        ...current.providers,
        [id]: { ...current.providers[id]!, ...patch },
      },
    }));
  };

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

  const handleRefresh = async (provider: ProviderSettingView) => {
    setBusy(provider.id);
    setError(null);
    setNotice(null);
    try {
      const catalog = await refreshProviderModels(provider.id, showAll[provider.id] ?? false, true);
      setCatalogs((current) => ({ ...current, [provider.id]: catalog }));
      if (catalog.degraded) {
        setNotice(catalog.message ?? 'The model list could not be refreshed.');
      }
    } catch (cause) {
      setError(toAppError(cause).message);
    } finally {
      setBusy(null);
    }
  };

  const handleTest = async (provider: ProviderSettingView) => {
    setBusy(provider.id);
    setError(null);
    setNotice(null);
    try {
      const result = await testProvider(
        provider.id,
        apiKeys[provider.id]?.trim() || undefined,
        draft.providers[provider.id]?.baseUrl || undefined,
        draft.providers[provider.id]?.model || undefined,
      );
      setNotice(
        result.ok
          ? `Connection ok (${result.latencyMs} ms).`
          : `Connection failed: ${result.message}`,
      );
    } catch (cause) {
      setError(toAppError(cause).message);
    } finally {
      setBusy(null);
    }
  };

  const handleSave = async () => {
    setBusy('save');
    setError(null);
    setNotice(null);
    const input: ProviderConfigInput = {
      order: draft.order,
      fallbackEnabled: draft.fallbackEnabled,
      providers: providers.map((provider) => {
        const value = draft.providers[provider.id]!;
        if (provider.id === 'openrouter') {
          return { id: provider.id, models: value.models };
        }
        return {
          id: provider.id,
          model: value.model || undefined,
          baseUrl: provider.id === 'ollama' ? value.baseUrl || undefined : undefined,
          attempts: value.attempts,
          enabled: provider.id === 'ollama' ? value.enabled : undefined,
        };
      }),
    };
    try {
      await state.save(input);
      setNotice('Provider settings saved.');
    } catch (cause) {
      setError(toAppError(cause).message);
    } finally {
      setBusy(null);
    }
  };

  const catalogFor = (provider: ProviderSettingView) => catalogs[provider.id]?.models ?? [];

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
          {notice ? (
            <Alert severity="info" sx={{ mb: 2 }}>
              {notice}
            </Alert>
          ) : null}

          <Stack spacing={1.5} sx={{ mb: 2 }}>
            <FormControlLabel
              control={
                <Switch
                  checked={draft.fallbackEnabled}
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      fallbackEnabled: event.target.checked,
                    }))
                  }
                />
              }
              label="Use fallback across providers"
            />
            <Typography variant="body2" color="text.secondary">
              Provider order: the first configured provider is used first.
            </Typography>
            <Stack spacing={0.75} data-testid="provider-order">
              {draft.order.map((providerId, index) => (
                <Box
                  key={providerId}
                  data-testid={`order-${providerId}`}
                  ref={(element: HTMLDivElement | null) => {
                    orderRows.current[index] = element;
                  }}
                  sx={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 1,
                    px: 1,
                    py: 0.5,
                    border: '1px solid',
                    borderColor: dragging === index ? 'primary.main' : 'divider',
                    borderRadius: 1,
                    bgcolor: dragging === index ? 'action.selected' : 'background.paper',
                    boxShadow: dragging === index ? 6 : 0,
                    transform: dragging === index ? 'scale(1.01)' : 'none',
                    zIndex: dragging === index ? 1 : 0,
                    position: 'relative',
                    transition:
                      'box-shadow 120ms ease, border-color 120ms ease, transform 120ms ease',
                  }}
                >
                  <Box
                    component="span"
                    role="button"
                    tabIndex={0}
                    aria-label={`Reorder ${providerId}`}
                    data-testid={`handle-${providerId}`}
                    onPointerDown={(event) => startDrag(event, index)}
                    onKeyDown={(event) => {
                      if (event.key === 'ArrowUp') moveProvider(index, index - 1);
                      if (event.key === 'ArrowDown') moveProvider(index, index + 1);
                    }}
                    sx={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      width: 24,
                      height: 24,
                      borderRadius: 1,
                      color: 'text.secondary',
                      cursor: 'grab',
                      touchAction: 'none',
                      userSelect: 'none',
                      '&:hover': { bgcolor: 'action.hover' },
                      '&:active': { cursor: 'grabbing' },
                    }}
                  >
                    ⠿
                  </Box>
                  <Typography sx={{ flex: 1, userSelect: 'none' }}>
                    {index + 1}. {labelFor(providerId)}
                  </Typography>
                  <Button
                    size="small"
                    aria-label={`Move ${providerId} up`}
                    disabled={index === 0}
                    onClick={() => moveProvider(index, index - 1)}
                  >
                    ↑
                  </Button>
                  <Button
                    size="small"
                    aria-label={`Move ${providerId} down`}
                    disabled={index === draft.order.length - 1}
                    onClick={() => moveProvider(index, index + 1)}
                  >
                    ↓
                  </Button>
                </Box>
              ))}
            </Stack>
          </Stack>

          <Stack spacing={1.5}>
            {providers.map((provider) => {
              const value = draft.providers[provider.id] ?? emptyDraft(provider);
              const catalog = catalogFor(provider);
              return (
                <Accordion
                  key={provider.id}
                  expanded={expanded[provider.id] ?? false}
                  onChange={(_, isExpanded) =>
                    setExpanded((current) => ({ ...current, [provider.id]: isExpanded }))
                  }
                  disableGutters
                >
                  <AccordionSummary expandIcon={<span aria-hidden>▾</span>}>
                    <Stack
                      direction="row"
                      spacing={1}
                      sx={{ width: '100%', alignItems: 'center', pr: 2 }}
                    >
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

                      {provider.id === 'ollama' ? (
                        <TextField
                          size="small"
                          label="Ollama base URL"
                          value={value.baseUrl}
                          onChange={(event) =>
                            updateProvider(provider.id, { baseUrl: event.target.value })
                          }
                        />
                      ) : null}

                      {provider.id === 'ollama' ? (
                        <FormControlLabel
                          control={
                            <Switch
                              checked={value.enabled}
                              disabled={catalog.length === 0}
                              onChange={(event) =>
                                updateProvider(provider.id, { enabled: event.target.checked })
                              }
                            />
                          }
                          label={
                            catalog.length === 0
                              ? 'Enable Ollama (no models found — run ollama pull <model>)'
                              : 'Include Ollama in the provider chain'
                          }
                        />
                      ) : null}

                      {provider.id === 'openrouter' ? (
                        <Stack spacing={1}>
                          <Stack direction="row" spacing={1} sx={{ flexWrap: 'wrap', gap: 1 }}>
                            {value.models.map((model) => (
                              <Chip
                                key={model}
                                label={model}
                                onDelete={() =>
                                  updateProvider(provider.id, {
                                    models: value.models.filter((item) => item !== model),
                                  })
                                }
                              />
                            ))}
                          </Stack>
                          <FormControl size="small" sx={{ minWidth: 240 }}>
                            <InputLabel id={`${provider.id}-add-model`}>Add model</InputLabel>
                            <Select
                              labelId={`${provider.id}-add-model`}
                              label="Add model"
                              value=""
                              onChange={(event) => {
                                const model = event.target.value as string;
                                if (!model || value.models.includes(model)) return;
                                updateProvider(provider.id, {
                                  models: [...value.models, model],
                                });
                              }}
                            >
                              {catalog
                                .filter((option) => !value.models.includes(option.id))
                                .map((option) => (
                                  <MenuItem key={option.id} value={option.id}>
                                    {option.id}
                                    {option.price ? ` — ${option.price}` : ''}
                                  </MenuItem>
                                ))}
                            </Select>
                          </FormControl>
                          <FormControlLabel
                            control={
                              <Switch
                                checked={showAll[provider.id] ?? false}
                                onChange={(event) => {
                                  const checked = event.target.checked;
                                  setShowAll((current) => ({
                                    ...current,
                                    [provider.id]: checked,
                                  }));
                                  void loadCatalog(provider.id, checked).catch(() => undefined);
                                }}
                              />
                            }
                            label="Show all models (including paid)"
                          />
                        </Stack>
                      ) : (
                        <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1}>
                          <FormControl size="small" sx={{ minWidth: 240 }}>
                            <InputLabel id={`${provider.id}-model`}>Model</InputLabel>
                            <Select
                              labelId={`${provider.id}-model`}
                              label="Model"
                              value={value.model}
                              onChange={(event) =>
                                updateProvider(provider.id, {
                                  model: event.target.value as string,
                                })
                              }
                            >
                              {catalog.map((option) => (
                                <MenuItem key={option.id} value={option.id}>
                                  {option.id}
                                </MenuItem>
                              ))}
                              {value.model && !catalog.some((item) => item.id === value.model) ? (
                                <MenuItem value={value.model}>{value.model}</MenuItem>
                              ) : null}
                            </Select>
                          </FormControl>
                          <TextField
                            size="small"
                            label="Custom model"
                            value={value.model}
                            onChange={(event) =>
                              updateProvider(provider.id, { model: event.target.value })
                            }
                            sx={{ flex: 1 }}
                          />
                        </Stack>
                      )}

                      <TextField
                        size="small"
                        type="number"
                        label="Attempts"
                        value={provider.id === 'openrouter' ? value.models.length : value.attempts}
                        disabled={provider.id === 'openrouter'}
                        onChange={(event) =>
                          updateProvider(provider.id, {
                            attempts: Number(event.target.value),
                          })
                        }
                        helperText={
                          provider.id === 'openrouter'
                            ? 'Each attempt uses the next model in the list'
                            : 'Attempts before this provider is considered failed'
                        }
                        sx={{ maxWidth: 320 }}
                      />

                      <Stack direction="row" spacing={1} sx={{ flexWrap: 'wrap', gap: 1 }}>
                        <Button
                          size="small"
                          disabled={busy === provider.id}
                          onClick={() => void handleRefresh(provider)}
                        >
                          Refresh models
                        </Button>
                        <Button
                          size="small"
                          disabled={busy === provider.id}
                          onClick={() => void handleTest(provider)}
                        >
                          Test connection
                        </Button>
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
              );
            })}
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={onClose}>Close</Button>
          <Button variant="contained" disabled={busy === 'save'} onClick={() => void handleSave()}>
            Save
          </Button>
        </DialogActions>
      </Dialog>

      {dragging !== null && draft.order[dragging] ? (
        <Box
          ref={dragGhost}
          aria-hidden
          data-testid="drag-ghost"
          sx={{
            position: 'fixed',
            top: 0,
            left: 0,
            pointerEvents: 'none',
            zIndex: (theme) => theme.zIndex.modal + 2,
            px: 1.5,
            py: 0.75,
            borderRadius: 1,
            border: '1px solid',
            borderColor: 'primary.main',
            bgcolor: 'background.paper',
            boxShadow: 6,
            opacity: 0.95,
            userSelect: 'none',
            whiteSpace: 'nowrap',
            willChange: 'transform',
          }}
        >
          ⠿ {draft.order[dragging] ? labelFor(draft.order[dragging]!) : ''}
        </Box>
      ) : null}

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
