import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { ProviderIndicator } from '@/components/ProviderIndicator';
import { providerConfigFixture, providerFixture } from '@/test/fixtures';
import type { ProviderConfigView } from '@/lib/types';

function renderIndicator(
  overrides: {
    active?: typeof providerFixture | null;
    error?: string | null;
    config?: ProviderConfigView | null;
  } = {},
) {
  const onSelectModel = vi.fn();
  const onOpenSettings = vi.fn();
  render(
    <ProviderIndicator
      active={overrides.active === undefined ? providerFixture : overrides.active}
      error={overrides.error ?? null}
      config={overrides.config === undefined ? providerConfigFixture : overrides.config}
      onSelectModel={onSelectModel}
      onOpenSettings={onOpenSettings}
    />,
  );
  return { onSelectModel, onOpenSettings };
}

describe('ProviderIndicator', () => {
  it('shows the active provider and model without credentials', () => {
    renderIndicator();
    expect(screen.getByRole('button', { name: 'Provider and model' })).toHaveTextContent(
      'deepseek · deepseek-flash',
    );
  });

  it('groups the configured models by provider', () => {
    renderIndicator();
    fireEvent.click(screen.getByRole('button', { name: 'Provider and model' }));

    expect(screen.getByText('gemini')).toBeInTheDocument();
    expect(screen.getByText('openrouter')).toBeInTheDocument();
    expect(screen.getByRole('menuitem', { name: 'gemini-3.5-flash-lite' })).toBeInTheDocument();
    expect(screen.getByRole('menuitem', { name: 'openrouter/free' })).toBeInTheDocument();
    expect(screen.getByRole('menuitem', { name: 'free/model' })).toBeInTheDocument();
    // DeepSeek has no credential in the fixture, but its model is still listed.
    expect(screen.getByRole('menuitem', { name: 'deepseek-flash' })).toBeInTheDocument();
  });

  it('selects a model for the session', () => {
    const { onSelectModel } = renderIndicator();
    fireEvent.click(screen.getByRole('button', { name: 'Provider and model' }));
    fireEvent.click(screen.getByRole('menuitem', { name: 'gemini-3.5-flash-lite' }));

    expect(onSelectModel).toHaveBeenCalledWith('gemini', ['gemini-3.5-flash-lite']);
  });

  it('offers to open settings when nothing is configured', () => {
    const { onOpenSettings } = renderIndicator({ active: null, config: null });
    fireEvent.click(screen.getByRole('button', { name: 'Provider and model' }));
    fireEvent.click(screen.getByRole('menuitem', { name: /No provider configured/ }));

    expect(onOpenSettings).toHaveBeenCalled();
  });

  it('surfaces a configuration error on the button', () => {
    renderIndicator({ active: null, error: 'no provider', config: null });
    expect(screen.getByRole('button', { name: 'Provider and model' })).toHaveTextContent(
      'Provider not configured',
    );
  });
});
