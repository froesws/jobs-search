import Tab from '@mui/material/Tab';
import Tabs from '@mui/material/Tabs';

import type { DashboardTab } from '@/viewmodels/useDashboardState';

const TABS: Array<{ value: DashboardTab; label: string }> = [
  { value: 'NEW_JOBS', label: 'New Jobs' },
  { value: 'APPLICATIONS', label: 'My Applications' },
  { value: 'PROFILE', label: 'My Profile' },
  { value: 'ANALYTICS', label: 'Analytics' },
];

export function TabBar({
  value,
  onChange,
}: {
  value: DashboardTab;
  onChange: (value: DashboardTab) => void;
}) {
  return (
    <Tabs
      value={value}
      onChange={(_, next: DashboardTab) => onChange(next)}
      variant="scrollable"
      allowScrollButtonsMobile
      sx={{
        minHeight: 48,
        borderBottom: '1px solid',
        borderColor: 'divider',
        px: 3,
        '& .MuiTab-root': {
          minHeight: 48,
          px: 2,
          mr: 1,
          fontSize: '0.88rem',
          fontWeight: 600,
          color: 'text.secondary',
        },
        '& .Mui-selected': { color: 'text.primary' },
      }}
    >
      {TABS.map((tab) => (
        <Tab key={tab.value} value={tab.value} label={tab.label} />
      ))}
    </Tabs>
  );
}
