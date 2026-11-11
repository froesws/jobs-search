import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import ListItemText from '@mui/material/ListItemText';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import Typography from '@mui/material/Typography';
import { useState } from 'react';
import type { MouseEvent } from 'react';

import type { ActiveProvider, ProviderConfigView, ProviderId } from '@/lib/types';

interface ModelChoice {
  provider: ProviderId;
  label: string;
  models: string[];
}

function choices(config: ProviderConfigView | null): ModelChoice[] {
  if (!config) return [];
  const result: ModelChoice[] = [];
  for (const providerId of config.order) {
    const provider = config.providers.find((item) => item.id === providerId);
    if (!provider) continue;
    if (provider.id === 'openrouter') {
      for (const model of provider.models ?? []) {
        result.push({ provider: provider.id, label: model, models: [model] });
      }
      continue;
    }
    if (provider.model) {
      result.push({ provider: provider.id, label: provider.model, models: [provider.model] });
    }
  }
  return result;
}

export function ProviderIndicator({
  active,
  error,
  config,
  onSelectModel,
  onOpenSettings,
}: {
  active: ActiveProvider | null;
  error: string | null;
  config: ProviderConfigView | null;
  onSelectModel: (provider: ProviderId, models: string[]) => void;
  onOpenSettings: () => void;
}) {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  const items = choices(config);

  const handleOpen = (event: MouseEvent<HTMLElement>) => {
    setAnchor(event.currentTarget);
  };

  const handleClose = () => setAnchor(null);

  const label = error
    ? 'Provider not configured'
    : active
      ? `${active.provider} · ${active.model}`
      : 'Provider…';

  return (
    <>
      <Button
        size="small"
        variant="outlined"
        color={error ? 'error' : active ? 'secondary' : 'primary'}
        aria-label="Provider and model"
        aria-haspopup="menu"
        onClick={handleOpen}
        sx={{ textTransform: 'none' }}
      >
        {label}
      </Button>
      <Menu anchorEl={anchor} open={anchor !== null} onClose={handleClose}>
        {items.length === 0 ? (
          <MenuItem
            onClick={() => {
              handleClose();
              onOpenSettings();
            }}
          >
            <ListItemText
              primary="No provider configured — open settings"
              secondary={error ?? undefined}
            />
          </MenuItem>
        ) : (
          buildMenuItems(items, handleClose, onSelectModel)
        )}
      </Menu>
    </>
  );
}

function buildMenuItems(
  items: ModelChoice[],
  close: () => void,
  onSelect: (provider: ProviderId, models: string[]) => void,
) {
  const nodes: React.ReactNode[] = [];
  let currentProvider: ProviderId | null = null;
  for (const item of items) {
    if (item.provider !== currentProvider) {
      if (currentProvider !== null) nodes.push(<Divider key={`divider-${item.provider}`} />);
      nodes.push(
        <MenuItem key={`header-${item.provider}`} disabled dense>
          <Typography variant="caption" color="text.secondary">
            {item.provider}
          </Typography>
        </MenuItem>,
      );
      currentProvider = item.provider;
    }
    nodes.push(
      <MenuItem
        key={`${item.provider}-${item.label}`}
        onClick={() => {
          close();
          onSelect(item.provider, item.models);
        }}
      >
        {item.label}
      </MenuItem>,
    );
  }
  return nodes;
}
