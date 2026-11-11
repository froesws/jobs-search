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

  it('does not crash when the configuration arrives after the first render', () => {
    const { rerender } = render(
      <ProviderConfigDialog open={false} onClose={() => {}} state={state({ config: null })} />,
    );

    expect(() =>
      rerender(<ProviderConfigDialog open={false} onClose={() => {}} state={state()} />),
    ).not.toThrow();
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

  it('derives OpenRouter attempts from the model list and omits the field', async () => {
    const save = vi.fn().mockResolvedValue(configView());
    const withOpenRouter = configView({
      providers: [
        {
          ...configView().providers[0]!,
          id: 'openrouter',
          label: 'OpenRouter',
          model: null,
          models: ['a', 'b'],
        },
      ],
    });
    render(
      <ProviderConfigDialog
        open
        onClose={() => {}}
        state={state({ config: withOpenRouter, save })}
      />,
    );

    fireEvent.click(screen.getByText('OpenRouter'));
    const attempts = screen.getByLabelText('Attempts');
    expect(attempts).toBeDisabled();
    expect(attempts).toHaveValue(2);

    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(save).toHaveBeenCalled());
    const input = vi.mocked(save).mock.calls[0]![0];
    expect(input.providers[0]).toEqual({ id: 'openrouter', models: ['a', 'b'] });
  });

  it('persists the provider order and fallback toggle', async () => {
    const save = vi.fn().mockResolvedValue(configView());
    render(<ProviderConfigDialog open onClose={() => {}} state={state({ save })} />);

    fireEvent.click(screen.getByRole('button', { name: 'Move ollama up' }));
    fireEvent.click(screen.getByLabelText('Use fallback across providers'));
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => expect(save).toHaveBeenCalled());
    const input = vi.mocked(save).mock.calls[0]![0];
    expect(input.fallbackEnabled).toBe(true);
    expect(input.order.indexOf('ollama')).toBeLessThan(input.order.indexOf('deepseek'));
  });

  it('reorders providers by dragging a row onto another', () => {
    render(<ProviderConfigDialog open onClose={() => {}} state={state()} />);

    fireEvent.dragStart(screen.getByTestId('order-ollama'));
    fireEvent.dragOver(screen.getByTestId('order-gemini'));
    fireEvent.drop(screen.getByTestId('order-gemini'));

    const rows = screen.getAllByTestId(/^order-/);
    expect(rows[0]).toHaveAttribute('data-testid', 'order-ollama');
    expect(rows[1]).toHaveAttribute('data-testid', 'order-gemini');
  });

  it('does not reorder while dragging over a row', () => {
    render(<ProviderConfigDialog open onClose={() => {}} state={state()} />);

    fireEvent.dragStart(screen.getByTestId('order-ollama'));
    fireEvent.dragOver(screen.getByTestId('order-gemini'));

    const rows = screen.getAllByTestId(/^order-/);
    expect(rows[0]).toHaveAttribute('data-testid', 'order-gemini');
    expect(rows[3]).toHaveAttribute('data-testid', 'order-ollama');
  });

  it('saves per-provider attempts within bounds', async () => {
    const save = vi.fn().mockResolvedValue(configView());
    render(<ProviderConfigDialog open onClose={() => {}} state={state({ save })} />);

    fireEvent.click(screen.getByText('Gemini'));
    const attempts = screen.getAllByLabelText('Attempts')[0]!;
    fireEvent.change(attempts, { target: { value: '5' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => expect(save).toHaveBeenCalled());
    const input = vi.mocked(save).mock.calls[0]![0];
    expect(input.providers).toEqual(
      expect.arrayContaining([expect.objectContaining({ id: 'gemini', attempts: 5 })]),
    );
  });

  it('disables enabling Ollama when no local models are available', async () => {
    vi.mocked(refreshProviderModels).mockResolvedValue(catalog({ provider: 'ollama', models: [] }));
    render(<ProviderConfigDialog open onClose={() => {}} state={state()} />);

    fireEvent.click(screen.getByText('Ollama (local)'));
    const toggle = screen.getByLabelText(
      'Enable Ollama (no models found — run ollama pull <model>)',
    );
    expect(toggle).toBeDisabled();
  });

  it('resets a field without removing the credential', async () => {
    const resetSetting = vi.fn().mockResolvedValue(configView());
    render(<ProviderConfigDialog open onClose={() => {}} state={state({ resetSetting })} />);

    fireEvent.click(screen.getByText('Gemini'));
    fireEvent.click(screen.getByRole('button', { name: 'Reset model' }));
    await waitFor(() => expect(resetSetting).toHaveBeenCalledWith('gemini', 'model'));
  });
});
