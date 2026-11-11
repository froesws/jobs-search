import { useCallback, useEffect, useState } from 'react';

import { safeMessage } from '@/lib/errors';
import {
  getActiveProvider,
  getProviderConfig,
  removeProviderCredential,
  resetProviderSetting,
  saveProviderConfig,
  saveProviderCredential,
} from '@/lib/ipc';
import type {
  ActiveProvider,
  ProviderConfigInput,
  ProviderConfigView,
  ProviderId,
  ProviderSettingField,
} from '@/lib/types';

export interface ProviderConfigState {
  config: ProviderConfigView | null;
  active: ActiveProvider | null;
  loading: boolean;
  error: string | null;
  reload: () => void;
  save: (input: ProviderConfigInput) => Promise<ProviderConfigView>;
  saveCredential: (
    provider: ProviderId,
    apiKey: string,
    allowEnvFallback?: boolean,
  ) => Promise<ProviderConfigView>;
  removeCredential: (provider: ProviderId, removeEnvEntry?: boolean) => Promise<ProviderConfigView>;
  resetSetting: (provider: ProviderId, field: ProviderSettingField) => Promise<ProviderConfigView>;
}

/**
 * Owns the provider configuration and the active provider shown in the top bar.
 * Saving or removing a credential refreshes both so the indicator never shows a
 * stale provider.
 */
export function useProviderConfig(): ProviderConfigState {
  const [config, setConfig] = useState<ProviderConfigView | null>(null);
  const [active, setActive] = useState<ActiveProvider | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [reloadToken, setReloadToken] = useState(0);

  const reload = useCallback(() => setReloadToken((value) => value + 1), []);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    Promise.all([getProviderConfig(), getActiveProvider()])
      .then(([nextConfig, nextActive]) => {
        if (cancelled) return;
        setConfig(nextConfig);
        setActive(nextActive);
        setError(null);
      })
      .catch((cause) => {
        if (cancelled) return;
        setConfig(null);
        setActive(null);
        setError(safeMessage(cause));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [reloadToken]);

  const apply = useCallback(async (next: ProviderConfigView) => {
    setConfig(next);
    try {
      setActive(await getActiveProvider());
    } catch {
      setActive(null);
    }
    return next;
  }, []);

  const save = useCallback(
    async (input: ProviderConfigInput) => apply(await saveProviderConfig(input)),
    [apply],
  );

  const saveCredential = useCallback(
    async (provider: ProviderId, apiKey: string, allowEnvFallback = false) =>
      apply(await saveProviderCredential(provider, apiKey, allowEnvFallback)),
    [apply],
  );

  const removeCredential = useCallback(
    async (provider: ProviderId, removeEnvEntry = false) =>
      apply(await removeProviderCredential(provider, removeEnvEntry)),
    [apply],
  );

  const resetSetting = useCallback(
    async (provider: ProviderId, field: ProviderSettingField) =>
      apply(await resetProviderSetting(provider, field)),
    [apply],
  );

  return {
    config,
    active,
    loading,
    error,
    reload,
    save,
    saveCredential,
    removeCredential,
    resetSetting,
  };
}
