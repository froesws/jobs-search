import Chip from '@mui/material/Chip';
import Tooltip from '@mui/material/Tooltip';
import { useEffect, useState } from 'react';

import { safeMessage } from '@/lib/errors';
import { getActiveProvider } from '@/lib/ipc';
import type { ActiveProvider } from '@/lib/types';

export function ProviderIndicator() {
  const [provider, setProvider] = useState<ActiveProvider | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    getActiveProvider()
      .then((value) => {
        if (active) setProvider(value);
      })
      .catch((cause) => {
        if (active) setError(safeMessage(cause));
      });
    return () => {
      active = false;
    };
  }, []);

  if (error) {
    return (
      <Tooltip title={error}>
        <Chip size="small" label="Provider not configured" color="error" variant="outlined" />
      </Tooltip>
    );
  }

  if (!provider) {
    return <Chip size="small" label="Provider…" variant="outlined" />;
  }

  return (
    <Tooltip
      title={`${provider.model}${provider.fallback_enabled ? ' · fallback enabled' : ' · fallback disabled'}`}
    >
      <Chip
        size="small"
        label={`${provider.provider} · ${provider.model}`}
        color="secondary"
        variant="outlined"
      />
    </Tooltip>
  );
}
