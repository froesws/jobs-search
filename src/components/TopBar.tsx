import type { ReactNode } from 'react';
import AppBar from '@mui/material/AppBar';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import CircularProgress from '@mui/material/CircularProgress';
import FormControl from '@mui/material/FormControl';
import IconButton from '@mui/material/IconButton';
import InputLabel from '@mui/material/InputLabel';
import MenuItem from '@mui/material/MenuItem';
import Select from '@mui/material/Select';
import TextField from '@mui/material/TextField';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Toolbar from '@mui/material/Toolbar';
import Tooltip from '@mui/material/Tooltip';
import Typography from '@mui/material/Typography';
import type { ThemeMode } from '@/theme/theme';
import { MODALITY_PANEL_VALUES, SENIORITY_PANEL_VALUES } from '@/lib/filters';
import type {
  ProviderId,
  SearchFilters,
  SearchTimeframe,
  Seniority,
  WorkModality,
} from '@/lib/types';
import { ProviderIndicator } from '@/components/ProviderIndicator';
import { ResumeManager } from '@/components/ResumeManager';
import type { ProviderConfigState } from '@/viewmodels/useProviderConfig';

const TIMEFRAMES: Array<{ value: SearchTimeframe; label: string }> = [
  { value: 'H24', label: 'Last 24 hours' },
  { value: 'D7', label: 'Last 7 days' },
  { value: 'D30', label: 'Last 30 days' },
];

const MODALITY_LABELS: Record<WorkModality, string> = {
  Remote: 'Remote',
  Hybrid: 'Hybrid',
  Onsite: 'On-site',
  Unknown: 'Unknown',
};

export function TopBar({
  filters,
  onChange,
  onSearch,
  searching,
  mode,
  onToggleMode,
  onOpenProviderSettings,
  provider,
  onSelectModel,
}: {
  filters: SearchFilters;
  onChange: (next: Partial<SearchFilters>) => void;
  onSearch: () => void;
  searching: boolean;
  mode: ThemeMode;
  onToggleMode: () => void;
  onOpenProviderSettings: () => void;
  provider: ProviderConfigState;
  onSelectModel: (provider: ProviderId, models: string[]) => void;
}) {
  return (
    <AppBar
      position="sticky"
      elevation={0}
      color="transparent"
      sx={{
        borderBottom: '1px solid',
        borderColor: 'divider',
        bgcolor: mode === 'dark' ? 'rgba(11, 15, 25, 0.72)' : 'rgba(244, 246, 251, 0.82)',
        backdropFilter: 'blur(18px)',
        zIndex: (theme) => theme.zIndex.drawer + 1,
      }}
    >
      <Toolbar sx={{ gap: 2, minHeight: 72 }}>
        <Box
          sx={{
            width: 38,
            height: 38,
            borderRadius: '12px',
            display: 'grid',
            placeItems: 'center',
            background: 'linear-gradient(135deg, #7c8cff 0%, #5eead4 100%)',
            color: '#0b0f19',
            fontWeight: 800,
            fontSize: 15,
            boxShadow: '0 8px 24px rgba(124,140,255,0.28)',
          }}
          aria-hidden
        >
          JS
        </Box>
        <Box sx={{ flex: 1, minWidth: 0 }}>
          <Typography variant="h1" sx={{ fontSize: '1.18rem', lineHeight: 1.15 }}>
            Jobs Search
          </Typography>
          <Typography variant="caption" color="text.secondary">
            Career intelligence workspace
          </Typography>
        </Box>

        <ProviderIndicator
          active={provider.active}
          error={provider.error}
          config={provider.config}
          onSelectModel={onSelectModel}
          onOpenSettings={onOpenProviderSettings}
        />
        <Tooltip title="Provider settings">
          <IconButton
            size="small"
            aria-label="Open provider settings"
            onClick={onOpenProviderSettings}
            sx={{ border: '1px solid', borderColor: 'divider' }}
          >
            <GearIcon />
          </IconButton>
        </Tooltip>
        <ResumeManager />

        <Tooltip title={mode === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}>
          <Button
            variant="outlined"
            size="small"
            onClick={onToggleMode}
            aria-label="Toggle color theme"
            sx={{ minWidth: 44, px: 1.5 }}
          >
            {mode === 'dark' ? 'Light' : 'Dark'}
          </Button>
        </Tooltip>

        <Button
          variant="contained"
          onClick={onSearch}
          disabled={searching}
          startIcon={searching ? <CircularProgress size={14} color="inherit" /> : undefined}
        >
          {searching ? 'Searching…' : 'Refresh Search'}
        </Button>
      </Toolbar>

      <Box
        data-testid="filter-row"
        sx={{
          display: 'flex',
          gap: 1.5,
          px: 3,
          pb: 2,
          pt: 2,
          flexWrap: 'wrap',
          alignItems: 'flex-start',
        }}
      >
        <FormControl size="small" sx={{ minWidth: 150, mt: '22px' }}>
          <InputLabel id="timeframe-label">Timeframe</InputLabel>
          <Select
            labelId="timeframe-label"
            label="Timeframe"
            value={filters.timeframe}
            onChange={(event) => onChange({ timeframe: event.target.value as SearchTimeframe })}
          >
            {TIMEFRAMES.map((item) => (
              <MenuItem key={item.value} value={item.value}>
                {item.label}
              </MenuItem>
            ))}
          </Select>
        </FormControl>

        <FilterGroup label="Seniority" showFooter={false} testId="seniority-footer">
          <ToggleButtonGroup
            exclusive={false}
            size="small"
            value={filters.seniorities}
            onChange={(_, value: Seniority[]) => onChange({ seniorities: value })}
            aria-label="Seniority"
            sx={{
              flexWrap: 'wrap',
              bgcolor: 'background.paper',
              borderRadius: 2,
              '& .MuiToggleButton-root': { height: 40 },
            }}
          >
            {SENIORITY_PANEL_VALUES.map((value) => (
              <ToggleButton key={value} value={value} sx={{ px: 1.5 }}>
                {value}
              </ToggleButton>
            ))}
          </ToggleButtonGroup>
        </FilterGroup>

        <TextField
          size="small"
          label="Stack or language"
          placeholder="Rust, React, C#"
          value={filters.stack_keyword}
          onChange={(event) => onChange({ stack_keyword: event.target.value })}
          sx={{ minWidth: 210, flex: '0 1 260px', mt: '22px' }}
        />

        <FilterGroup label="Work type" showFooter={false} testId="modality-footer">
          <ToggleButtonGroup
            exclusive={false}
            size="small"
            value={filters.modalities}
            onChange={(_, value: WorkModality[]) => onChange({ modalities: value })}
            aria-label="Work modality"
            sx={{
              flexWrap: 'wrap',
              bgcolor: 'background.paper',
              borderRadius: 2,
              '& .MuiToggleButton-root': { height: 40 },
            }}
          >
            {MODALITY_PANEL_VALUES.map((value) => (
              <ToggleButton key={value} value={value} sx={{ px: 1.5 }}>
                {MODALITY_LABELS[value]}
              </ToggleButton>
            ))}
          </ToggleButtonGroup>
        </FilterGroup>
      </Box>
    </AppBar>
  );
}

function GearIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden focusable="false">
      <path
        d="M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Z"
        stroke="currentColor"
        strokeWidth="1.6"
      />
      <path
        d="M19.4 13.5a7.7 7.7 0 0 0 0-3l1.7-1.3-1.8-3.1-2 .8a7.6 7.6 0 0 0-2.6-1.5L14.4 3h-3.6l-.3 2.4a7.6 7.6 0 0 0-2.6 1.5l-2-.8-1.8 3.1 1.7 1.3a7.7 7.7 0 0 0 0 3l-1.7 1.3 1.8 3.1 2-.8a7.6 7.6 0 0 0 2.6 1.5l.3 2.4h3.6l.3-2.4a7.6 7.6 0 0 0 2.6-1.5l2 .8 1.8-3.1-1.7-1.3Z"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function FilterGroup({
  label,
  showFooter,
  testId,
  children,
}: {
  label: string;
  showFooter: boolean;
  testId: string;
  children: ReactNode;
}) {
  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.25, minWidth: 0 }}>
      <Typography variant="caption" color="text.secondary">
        {label}
      </Typography>
      {children}
      <Typography
        variant="caption"
        color="text.secondary"
        data-testid={testId}
        aria-hidden={showFooter ? undefined : true}
        style={{ visibility: showFooter ? 'visible' : 'hidden' }}
      >
        Profile default
      </Typography>
    </Box>
  );
}
