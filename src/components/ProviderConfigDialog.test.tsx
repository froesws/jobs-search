import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { AppError } from '@/lib/errors';
import { refreshProviderModels, testProvider } from '@/lib/ipc';
import type { ModelCatalogView, ProviderConfigView } from '@/lib/types';
import { ProviderConfigDialog } from '@/components/ProviderConfigDialog';
import type { ProviderConfigState } from '@/viewmodels/useProviderConfig';

vi.mock('@/lib/ipc', () => ({
  refreshProviderModels: vi.fn(),
  testProvider: vi.fn(),
}));

function configView(overrides: Partial<ProviderConfigView> = {}): ProviderConfigView {
  return {
    providers: [
      {
        id: 'gemini',
        label: 'Gemini',
        requiresApiKey: true,
        hasCredential: false,
        credentialSource: 'none',
        model: 'gemini-3.5-flash-lite',
        models: null,
        baseUrl: null,
        attempts: 3,
        enabled: true,
      },
      {
        id: 'ollama',
        label: 'Ollama (local)',
        requiresApiKey: false,
        hasCredential: false,
        credentialSource: 'none',
        model: null,
        models: null,
        baseUrl: 'http://localhost:11434',
        attempts: 2,
        enabled: false,
      },
    ],
    order: ['gemini', 'openrouter', 'deepseek', 'ollama'],
    fallbackEnabled: false,
    sessionOverride: null,
    ...overrides,
  };
}

function state(overrides: Partial<ProviderConfigState> = {}): ProviderConfigState {
  return {
    config: configView(),
    active: null,
    loading: false,
    error: null,
    reload: vi.fn(),
    save: vi.fn(),
    saveCredential: vi.fn().mockResolvedValue(configView()),
    removeCredential: vi.fn().mockResolvedValue(configView()),
    resetSetting: vi.fn().mockResolvedValue(configView()),
    ...overrides,
  };
}

function catalog(overrides: Partial<ModelCatalogView> = {}): ModelCatalogView {
  return {
    provider: 'gemini',
    models: [{ id: 'gemini-3.5-flash-lite', price: null }],
    source: 'cache',
    fetchedAt: null,
    degraded: false,
    message: null,
    ...overrides,
  };
}

describe('ProviderConfigDialog', () => {
  beforeEach(() => {
    vi.mocked(refreshProviderModels).mockResolvedValue(catalog());
    vi.mocked(testProvider).mockResolvedValue({
      ok: true,
      latencyMs: 12,
      message: 'connection ok',
    });
  });

  it('shows the configured state without revealing a key', () => {
    render(<ProviderConfigDialog open onClose={() => {}} state={state()} />);

    expect(screen.getByText('Gemini')).toBeInTheDocument();
    expect(screen.getAllByText('not configured')).toHaveLength(2);
    expect(screen.queryByDisplayValue(/secret/i)).not.toBeInTheDocument();
  });

  it('saves a credential for a provider', async () => {
    const saveCredential = vi.fn().mockResolvedValue(configView());
    render(<ProviderConfigDialog open onClose={() => {}} state={state({ saveCredential })} />);

    fireEvent.click(screen.getByText('Gemini'));
    fireEvent.change(screen.getByLabelText('Gemini API key'), {
      target: { value: 'new-key' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save key' }));

    await waitFor(() => expect(saveCredential).toHaveBeenCalledWith('gemini', 'new-key', false));
  });

  it('asks for consent before writing to .env', async () => {
    const saveCredential = vi
      .fn()
      .mockRejectedValueOnce(new AppError('CREDENTIAL_STORE_UNAVAILABLE', 'store unavailable'))
      .mockResolvedValueOnce(configView());
    render(<ProviderConfigDialog open onClose={() => {}} state={state({ saveCredential })} />);

    fireEvent.click(screen.getByText('Gemini'));
    fireEvent.change(screen.getByLabelText('Gemini API key'), {
      target: { value: 'new-key' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save key' }));

    expect(await screen.findByText('Credential store unavailable')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Write to .env' }));

    await waitFor(() => expect(saveCredential).toHaveBeenLastCalledWith('gemini', 'new-key', true));
  });

  it('cancelling the consent prompt writes nothing', async () => {
    const saveCredential = vi
      .fn()
      .mockRejectedValue(new AppError('CREDENTIAL_STORE_UNAVAILABLE', 'store unavailable'));
    render(<ProviderConfigDialog open onClose={() => {}} state={state({ saveCredential })} />);

    fireEvent.click(screen.getByText('Gemini'));
    fireEvent.change(screen.getByLabelText('Gemini API key'), {
      target: { value: 'new-key' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save key' }));
    await screen.findByText('Credential store unavailable');
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));

    expect(saveCredential).toHaveBeenCalledTimes(1);
  });

  it('removes a credential without touching .env by default', async () => {
    const removeCredential = vi.fn().mockResolvedValue(configView());
    const configured = configView({
      providers: [
        {
          ...configView().providers[0]!,
          hasCredential: true,
          credentialSource: 'credential_store',
        },
      ],
    });
    render(
      <ProviderConfigDialog
        open
        onClose={() => {}}
        state={state({ config: configured, removeCredential })}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Remove credential' }));
    await waitFor(() => expect(removeCredential).toHaveBeenCalledWith('gemini', false));
  });

  it('refreshes the model catalog and reports a degraded result', async () => {
    vi.mocked(refreshProviderModels).mockResolvedValue(
      catalog({ degraded: true, message: 'refresh failed' }),
    );
    render(<ProviderConfigDialog open onClose={() => {}} state={state()} />);

    fireEvent.click(screen.getByText('Gemini'));
    fireEvent.click(screen.getByRole('button', { name: 'Refresh models' }));

    expect(await screen.findByText('refresh failed')).toBeInTheDocument();
  });

  it('requests paid models when "show all" is enabled', async () => {
    render(<ProviderConfigDialog open onClose={() => {}} state={state()} />);

    fireEvent.click(screen.getByText('Gemini'));
    fireEvent.click(screen.getByRole('button', { name: 'Test connection' }));
    await waitFor(() => expect(testProvider).toHaveBeenCalled());

    expect(refreshProviderModels).toHaveBeenCalledWith('gemini', false, false);
  });

  it('saves a custom model identifier', async () => {
    const save = vi.fn().mockResolvedValue(configView());
    render(<ProviderConfigDialog open onClose={() => {}} state={state({ save })} />);

    fireEvent.click(screen.getByText('Gemini'));
    fireEvent.change(screen.getAllByLabelText('Custom model')[0]!, {
      target: { value: 'gemini-custom' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => expect(save).toHaveBeenCalled());
    const input = vi.mocked(save).mock.calls[0]![0];
    expect(input.providers).toEqual(
      expect.arrayContaining([expect.objectContaining({ id: 'gemini', model: 'gemini-custom' })]),
    );
  });

  it('resets a field without removing the credential', async () => {
    const resetSetting = vi.fn().mockResolvedValue(configView());
    render(<ProviderConfigDialog open onClose={() => {}} state={state({ resetSetting })} />);

    fireEvent.click(screen.getByText('Gemini'));
    fireEvent.click(screen.getByRole('button', { name: 'Reset model' }));
    await waitFor(() => expect(resetSetting).toHaveBeenCalledWith('gemini', 'model'));
  });
});
