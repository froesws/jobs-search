import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { FeedbackProvider } from '../components/FeedbackProvider';
import { getProfile, saveProfile } from '../lib/ipc';
import type { CandidateProfile } from '../lib/types';
import { ProfileView } from './ProfileView';

vi.mock('../lib/ipc', () => ({
  getProfile: vi.fn(),
  saveProfile: vi.fn(),
}));

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
