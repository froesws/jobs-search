import Autocomplete from '@mui/material/Autocomplete';
import Alert from '@mui/material/Alert';
import Box from '@mui/material/Box';
import Button from '@mui/material/Button';
import Chip from '@mui/material/Chip';
import Paper from '@mui/material/Paper';
import TextField from '@mui/material/TextField';
import ToggleButton from '@mui/material/ToggleButton';
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup';
import Typography from '@mui/material/Typography';
import { useEffect, useState } from 'react';

import { safeMessage } from '../lib/errors';
import type { CandidateProfile, WorkModality } from '../lib/types';
import { useFeedback } from '../components/FeedbackProvider';
import { ErrorState } from '../components/ErrorState';
import { LoadingState } from '../components/LoadingState';
import { useProfileViewModel } from '../viewmodels/useProfileViewModel';

const SENIORITIES = ['Trainee', 'Intern', 'Junior', 'Pleno', 'Senior'];
const MODALITIES: WorkModality[] = ['Remote', 'Hybrid', 'Onsite'];

export function ProfileView() {
  const { status, profile, error, saving, refresh, save } = useProfileViewModel();
  const feedback = useFeedback();
  const [draft, setDraft] = useState<CandidateProfile | null>(null);

  useEffect(() => {
    if (profile) setDraft(profile);
  }, [profile]);

  if (status === 'loading' || status === 'idle') {
    return <LoadingState label="Loading your candidate profile…" />;
  }
  if (status === 'error' || !profile) {
    return <ErrorState message={error ?? 'No profile is available.'} onRetry={refresh} />;
  }
  if (!draft) return <LoadingState label="Preparing the editor…" />;

  const conflicts = Array.isArray((draft.extras as { conflicts?: unknown })?.conflicts)
    ? ((draft.extras as { conflicts: Array<{ field: string; values: string[] }> }).conflicts ?? [])
    : [];

  const update = (patch: Partial<CandidateProfile>) => setDraft({ ...draft, ...patch });

  const handleSave = async () => {
    try {
      await save(draft);
      feedback.notify('Profile saved. New searches will use this version.', 'success');
    } catch (cause) {
      feedback.notify(safeMessage(cause), 'error', true);
    }
  };

  return (
    <Box sx={{ display: 'grid', gap: 2 }}>
      <Box sx={{ display: 'flex', alignItems: 'flex-end', gap: 2, flexWrap: 'wrap' }}>
        <Box sx={{ flex: 1 }}>
          <Typography variant="h1">My Profile</Typography>
          <Typography color="text.secondary" variant="body2">
            Review extracted data and edit the constraints that cannot be inferred from a resume.
          </Typography>
        </Box>
        <Chip label={`Profile v${profile.version}`} color="primary" variant="outlined" />
        <Button variant="outlined" onClick={() => setDraft(profile)} disabled={saving}>
          Discard
        </Button>
        <Button variant="contained" onClick={() => void handleSave()} disabled={saving}>
          {saving ? 'Saving…' : 'Save profile'}
        </Button>
      </Box>

      {conflicts.length > 0 ? (
        <Alert severity="warning" variant="outlined">
          {conflicts.length} conflicting value(s) need review:{' '}
          {conflicts
            .map((conflict) => `${conflict.field} (${conflict.values.join(' vs ')})`)
            .join(' · ')}
        </Alert>
      ) : null}

      <Box
        sx={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(330px, 1fr))',
          gap: 2,
        }}
      >
        <EditorCard title="Target roles">
          <Autocomplete
            multiple
            freeSolo
            options={[]}
            value={draft.target_roles}
            onChange={(_, value) => update({ target_roles: value })}
            renderInput={(params) => <TextField {...params} label="Roles" size="small" />}
          />
        </EditorCard>

        <EditorCard title="Seniority range">
          <Autocomplete
            multiple
            options={SENIORITIES}
            value={draft.seniority_range}
            onChange={(_, value) => update({ seniority_range: value })}
            renderInput={(params) => (
              <TextField {...params} label="Accepted seniorities" size="small" />
            )}
          />
        </EditorCard>

        <EditorCard title="Work modalities">
          <ToggleButtonGroup
            value={draft.work_modalities}
            onChange={(_, value: string[]) => update({ work_modalities: value })}
            aria-label="Preferred work modalities"
            size="small"
          >
            {MODALITIES.map((modality) => (
              <ToggleButton key={modality} value={modality}>
                {modality}
              </ToggleButton>
            ))}
          </ToggleButtonGroup>
        </EditorCard>

        <EditorCard title="Accepted cities">
          <Autocomplete
            multiple
            freeSolo
            options={['Porto Alegre', 'Curitiba']}
            value={draft.accepted_cities}
            onChange={(_, value) => update({ accepted_cities: value })}
            renderInput={(params) => (
              <TextField {...params} label="On-site / hybrid cities" size="small" />
            )}
          />
        </EditorCard>

        <EditorCard title="Acceptable technologies">
          <Autocomplete
            multiple
            freeSolo
            options={[]}
            value={draft.acceptable_technologies}
            onChange={(_, value) => update({ acceptable_technologies: value })}
            renderInput={(params) => (
              <TextField {...params} label="Manual technology tags" size="small" />
            )}
          />
          <Typography variant="caption" color="text.secondary">
            Merged into core competencies and used by the hard filter and core-skills score.
          </Typography>
        </EditorCard>

        <EditorCard title="Dealbreakers">
          <Autocomplete
            multiple
            freeSolo
            options={[]}
            value={draft.dealbreakers}
            onChange={(_, value) => update({ dealbreakers: value })}
            renderInput={(params) => (
              <TextField {...params} label="Dealbreaker phrases" size="small" />
            )}
          />
        </EditorCard>

        <EditorCard title="Constraints">
          <TextField
            label="Salary threshold"
            type="number"
            size="small"
            value={draft.salary_threshold ?? ''}
            onChange={(event) =>
              update({
                salary_threshold: event.target.value ? Number(event.target.value) : null,
              })
            }
          />
          <TextField
            label="Work authorization / visa notes"
            size="small"
            value={draft.work_authorization ?? ''}
            onChange={(event) => update({ work_authorization: event.target.value || null })}
          />
          <Typography variant="caption" color="text.secondary">
            Salary and work authorization are preferences, not hard filters in V1.
          </Typography>
        </EditorCard>

        <ReadOnlyCard
          title="Extracted core competencies"
          values={draft.core_competencies}
          color="primary"
        />
        <ReadOnlyCard title="Secondary competencies" values={draft.secondary_competencies} />
        <ReadOnlyCard title="Domain experience" values={draft.domain_experience} />
        <ReadOnlyCard title="Certifications" values={readExtras(draft, 'certifications')} />
        <ReadOnlyCard title="Source documents" values={draft.source_documents} />
      </Box>
    </Box>
  );
}

function readExtras(profile: CandidateProfile, key: string): string[] {
  const value = (profile.extras as Record<string, unknown> | undefined)?.[key];
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === 'string')
    : [];
}

function EditorCard({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <Paper sx={{ p: 2.5, display: 'grid', gap: 1.5, alignContent: 'start' }}>
      <Typography variant="h3">{title}</Typography>
      {children}
    </Paper>
  );
}

function ReadOnlyCard({
  title,
  values,
  color = 'default',
}: {
  title: string;
  values: string[];
  color?: 'default' | 'primary';
}) {
  return (
    <Paper sx={{ p: 2.5, display: 'grid', gap: 1.5, alignContent: 'start' }}>
      <Typography variant="h3">{title}</Typography>
      {values.length === 0 ? (
        <Typography variant="body2" color="text.secondary">
          Not extracted yet.
        </Typography>
      ) : (
        <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.75 }}>
          {values.map((value) => (
            <Chip key={value} size="small" label={value} color={color} variant="outlined" />
          ))}
        </Box>
      )}
    </Paper>
  );
}
