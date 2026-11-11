import AppBar from '@mui/material/AppBar';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import CircularProgress from '@mui/material/CircularProgress';
import FormControl from '@mui/material/FormControl';
import InputLabel from '@mui/material/InputLabel';
import MenuItem from '@mui/material/MenuItem';
import Select from '@mui/material/Select';
import TextField from '@mui/material/TextField';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Toolbar from '@mui/material/Toolbar';
import Tooltip from '@mui/material/Tooltip';
import Typography from '@mui/material/Typography';
import type { ThemeMode } from '../theme/theme';
import type { SearchFilters, SearchTimeframe, Seniority, WorkModality } from '../lib/types';
import { ProviderIndicator } from './ProviderIndicator';
import { ResumeManager } from './ResumeManager';

const TIMEFRAMES: Array<{ value: SearchTimeframe; label: string }> = [
  { value: 'H24', label: 'Last 24 hours' },
  { value: 'D7', label: 'Last 7 days' },
  { value: 'D30', label: 'Last 30 days' },
];

const SENIORITIES: Seniority[] = ['Trainee', 'Intern', 'Junior', 'Pleno', 'Senior'];
const MODALITIES: Array<{ value: 'All' | WorkModality; label: string }> = [
  { value: 'All', label: 'All' },
  { value: 'Remote', label: 'Remote' },
  { value: 'Hybrid', label: 'Hybrid' },
  { value: 'Onsite', label: 'On-site' },
];

export function TopBar({
  filters,
  onChange,
  onSearch,
  searching,
  mode,
  onToggleMode,
}: {
  filters: SearchFilters;
  onChange: (next: Partial<SearchFilters>) => void;
  onSearch: () => void;
  searching: boolean;
  mode: ThemeMode;
  onToggleMode: () => void;
}) {
  const seniority = filters.seniorities[0] ?? 'Any';
  const modality = filters.modalities[0] ?? 'All';

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

        <ProviderIndicator />
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
        sx={{
          display: 'flex',
          gap: 1.5,
          alignItems: 'center',
          px: 3,
          pb: 2,
          pt: 0.5,
          flexWrap: 'wrap',
        }}
      >
        <FormControl size="small" sx={{ minWidth: 150 }}>
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

        <FormControl size="small" sx={{ minWidth: 130 }}>
          <InputLabel id="seniority-label">Seniority</InputLabel>
          <Select
            labelId="seniority-label"
            label="Seniority"
            value={seniority}
            onChange={(event) => {
              const value = event.target.value as Seniority | 'Any';
              onChange({ seniorities: value === 'Any' ? [] : [value] });
            }}
          >
            <MenuItem value="Any">Any</MenuItem>
            {SENIORITIES.map((value) => (
              <MenuItem key={value} value={value}>
                {value}
              </MenuItem>
            ))}
          </Select>
        </FormControl>

        <TextField
          size="small"
          label="Stack or language"
          placeholder="Rust, React, C#"
          value={filters.stack_keyword}
          onChange={(event) => onChange({ stack_keyword: event.target.value })}
          sx={{ minWidth: 210, flex: '0 1 260px' }}
        />

        <ToggleButtonGroup
          exclusive
          size="small"
          value={modality}
          onChange={(_, value: 'All' | WorkModality | null) => {
            if (!value) return;
            onChange({ modalities: value === 'All' ? [] : [value] });
          }}
          aria-label="Work modality"
          sx={{ bgcolor: 'background.paper', borderRadius: 2 }}
        >
          {MODALITIES.map((item) => (
            <ToggleButton key={item.value} value={item.value} sx={{ px: 1.75 }}>
              {item.label}
            </ToggleButton>
          ))}
        </ToggleButtonGroup>
      </Box>
    </AppBar>
  );
}
