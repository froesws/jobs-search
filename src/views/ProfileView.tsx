import Box from '@mui/material/Box';
import Chip from '@mui/material/Chip';
import Paper from '@mui/material/Paper';
import Typography from '@mui/material/Typography';

import { useProfileViewModel } from '../viewmodels/useProfileViewModel';
import { ErrorState } from '../components/ErrorState';
import { LoadingState } from '../components/LoadingState';

export function ProfileView() {
  const { status, profile, error, refresh } = useProfileViewModel();

  if (status === 'loading' || status === 'idle') {
    return <LoadingState label="Loading your candidate profile…" />;
  }
  if (status === 'error' || !profile) {
    return <ErrorState message={error ?? 'No profile is available.'} onRetry={refresh} />;
  }

  return (
    <Box sx={{ display: 'grid', gap: 2 }}>
      <Box sx={{ display: 'flex', alignItems: 'flex-end', gap: 2, flexWrap: 'wrap' }}>
        <Box sx={{ flex: 1 }}>
          <Typography variant="h1">My Profile</Typography>
          <Typography color="text.secondary" variant="body2">
            The verified profile used by every evaluation. Editing arrives with the profile
            milestone.
          </Typography>
        </Box>
        <Chip label={`Profile v${profile.version}`} color="primary" variant="outlined" />
        <Chip label="Read-only in this milestone" variant="outlined" />
      </Box>

      <Box
        sx={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))',
          gap: 2,
        }}
      >
        <ProfileCard title="Target roles">
          <Chips values={profile.target_roles} color="primary" />
        </ProfileCard>
        <ProfileCard title="Seniority & modalities">
          <Chips values={profile.seniority_range} />
          <Chips values={profile.work_modalities} color="secondary" />
        </ProfileCard>
        <ProfileCard title="Accepted cities">
          <Chips values={profile.accepted_cities} color="secondary" />
        </ProfileCard>
        <ProfileCard title="Core competencies">
          <Chips values={profile.core_competencies} color="primary" />
        </ProfileCard>
        <ProfileCard title="Acceptable technologies (manual)">
          <Chips values={profile.acceptable_technologies} />
        </ProfileCard>
        <ProfileCard title="Secondary competencies">
          <Chips values={profile.secondary_competencies} />
        </ProfileCard>
        <ProfileCard title="Domain experience">
          <Chips values={profile.domain_experience} />
        </ProfileCard>
        <ProfileCard title="Dealbreakers">
          {profile.dealbreakers.length === 0 ? (
            <Muted>No dealbreakers configured.</Muted>
          ) : (
            <Chips values={profile.dealbreakers} color="error" />
          )}
        </ProfileCard>
        <ProfileCard title="Constraints">
          <Box sx={{ display: 'grid', gap: 1 }}>
            <KeyValue label="Work authorization" value={profile.work_authorization ?? 'Not set'} />
            <KeyValue
              label="Salary threshold"
              value={profile.salary_threshold ? `${profile.salary_threshold}` : 'Not set'}
            />
            <KeyValue
              label="Mandatory criteria"
              value={
                profile.mandatory_criteria.length ? profile.mandatory_criteria.join(', ') : 'None'
              }
            />
          </Box>
        </ProfileCard>
        <ProfileCard title="Source documents">
          {profile.source_documents.length === 0 ? (
            <Muted>This fixture profile has no resume lineage yet.</Muted>
          ) : (
            <Chips values={profile.source_documents} />
          )}
        </ProfileCard>
      </Box>
    </Box>
  );
}

function ProfileCard({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <Paper sx={{ p: 2.5, display: 'grid', gap: 1.5, alignContent: 'start' }}>
      <Typography variant="h3">{title}</Typography>
      {children}
    </Paper>
  );
}

function Chips({
  values,
  color = 'default',
}: {
  values: string[];
  color?: 'default' | 'primary' | 'secondary' | 'error';
}) {
  return (
    <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.75 }}>
      {values.map((value) => (
        <Chip key={value} size="small" label={value} color={color} variant="outlined" />
      ))}
    </Box>
  );
}

function KeyValue({ label, value }: { label: string; value: string }) {
  return (
    <Box sx={{ display: 'flex', justifyContent: 'space-between', gap: 2 }}>
      <Typography variant="caption" color="text.secondary">
        {label}
      </Typography>
      <Typography variant="body2" sx={{ fontWeight: 600, textAlign: 'right' }}>
        {value}
      </Typography>
    </Box>
  );
}

function Muted({ children }: { children: React.ReactNode }) {
  return (
    <Typography variant="body2" color="text.secondary">
      {children}
    </Typography>
  );
}
