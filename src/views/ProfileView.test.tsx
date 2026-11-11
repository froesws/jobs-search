import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { FeedbackProvider } from '@/components/FeedbackProvider';
import { getProfile, saveProfile } from '@/lib/ipc';
import type { CandidateProfile } from '@/lib/types';
import { ProfileView } from '@/views/ProfileView';

vi.mock('@/lib/ipc', async () => {
  const { createIpcMock } = await import('@/test/ipc-mock');
  return createIpcMock();
});

const profile: CandidateProfile = {
  profile_id: 'p1',
  version: 3,
  updated_at: '2026-11-10T00:00:00Z',
  target_roles: ['Rust Engineer'],
  seniority_range: ['Pleno'],
  work_modalities: ['Remote'],
  accepted_cities: ['Porto Alegre'],
  acceptable_technologies: ['Rust'],
  core_competencies: ['Rust'],
  secondary_competencies: ['SQL'],
  domain_experience: ['systems'],
  dealbreakers: [],
  mandatory_criteria: ['location'],
  work_authorization: null,
  salary_threshold: null,
  source_documents: [],
};

describe('ProfileView', () => {
  beforeEach(() => {
    vi.mocked(getProfile).mockResolvedValue(profile);
    vi.mocked(saveProfile).mockResolvedValue({ ...profile, version: 4 });
  });

  it('renders the verified profile and saves edits', async () => {
    render(
      <FeedbackProvider>
        <ProfileView />
      </FeedbackProvider>,
    );

    expect(await screen.findByText('My Profile')).toBeInTheDocument();
    expect(screen.getByText('Profile v3')).toBeInTheDocument();
    expect(screen.getByText('Acceptable technologies')).toBeInTheDocument();
    expect(screen.getAllByText('Rust').length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole('button', { name: 'Save profile' }));
    await waitFor(() => expect(saveProfile).toHaveBeenCalledTimes(1));
    expect(vi.mocked(saveProfile).mock.calls[0]?.[0].profile_id).toBe('p1');
    expect(await screen.findByText(/Profile saved/)).toBeInTheDocument();
  });
});

const longValue =
  'Principal_Rust_Engineer_With_An_Extremely_Long_Unbroken_Title_That_Must_Wrap_Inside_The_Card';

const longProfile: CandidateProfile = {
  ...profile,
  target_roles: [longValue],
  accepted_cities: [longValue],
  acceptable_technologies: [longValue],
  dealbreakers: [longValue],
  work_authorization: `${longValue} ${longValue}`,
  core_competencies: [longValue],
  secondary_competencies: [longValue],
  domain_experience: [longValue],
  extras: {
    resumes_available: true,
    certifications: [longValue],
    conflicts: [{ field: 'custom', values: [longValue], sources: ['resume-1'] }],
  },
};

describe('ProfileView long values', () => {
  it('wraps long values without forcing cards to overflow', async () => {
    vi.mocked(getProfile).mockResolvedValue(longProfile);

    render(
      <FeedbackProvider>
        <ProfileView />
      </FeedbackProvider>,
    );

    expect(await screen.findByText('My Profile')).toBeInTheDocument();
    expect(screen.getAllByText(longValue).length).toBeGreaterThan(3);

    const cards = screen.getAllByTestId('profile-card');
    expect(cards.length).toBeGreaterThan(0);
    for (const card of cards) {
      expect(card).toHaveStyle({ minWidth: '0px', overflowWrap: 'anywhere' });
    }

    const chips = screen.getAllByTestId('profile-chip');
    expect(chips.length).toBeGreaterThan(0);
    for (const chip of chips) {
      expect(chip).toHaveStyle({ maxWidth: '100%', height: 'auto' });
    }

    const notes = screen.getByLabelText('Work authorization / visa notes');
    expect(notes.tagName).toBe('TEXTAREA');
    expect(screen.getByTestId('profile-conflicts')).toHaveStyle({
      overflowWrap: 'anywhere',
    });
  });
});
