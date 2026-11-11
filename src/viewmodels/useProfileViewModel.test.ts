import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getProfile, saveProfile } from '../lib/ipc';
import type { CandidateProfile } from '../lib/types';
import { useProfileViewModel } from './useProfileViewModel';

vi.mock('../lib/ipc', async () => {
  const { createIpcMock } = await import('../test/ipc-mock');
  return createIpcMock();
});

const profile: CandidateProfile = {
  profile_id: 'p1',
  version: 1,
  updated_at: '2026-11-10T00:00:00Z',
  target_roles: ['Rust Engineer'],
  seniority_range: ['Pleno'],
  work_modalities: ['Remote'],
  accepted_cities: ['Porto Alegre'],
  acceptable_technologies: ['Rust'],
  core_competencies: ['Rust'],
  secondary_competencies: [],
  domain_experience: [],
  dealbreakers: [],
  mandatory_criteria: ['location'],
  work_authorization: null,
  salary_threshold: null,
  source_documents: [],
};

describe('useProfileViewModel', () => {
  beforeEach(() => {
    vi.mocked(getProfile).mockReset();
    vi.mocked(saveProfile).mockReset();
  });

  it('loads the active profile', async () => {
    vi.mocked(getProfile).mockResolvedValue(profile);
    const { result } = renderHook(() => useProfileViewModel());

    expect(result.current.status).toBe('loading');
    await waitFor(() => expect(result.current.status).toBe('ready'));
    expect(result.current.profile?.profile_id).toBe('p1');
  });

  it('saves and replaces the in-memory profile', async () => {
    vi.mocked(getProfile).mockResolvedValue(profile);
    const saved = { ...profile, version: 2, acceptable_technologies: ['Rust', 'React'] };
    vi.mocked(saveProfile).mockResolvedValue(saved);
    const { result } = renderHook(() => useProfileViewModel());
    await waitFor(() => expect(result.current.status).toBe('ready'));

    await result.current.save(profile);
    await waitFor(() => expect(result.current.profile?.version).toBe(2));
    expect(result.current.saving).toBe(false);
  });

  it('surfaces a profile load error', async () => {
    vi.mocked(getProfile).mockRejectedValue({ code: 'PROFILE_REQUIRED', message: 'no profile' });
    const { result } = renderHook(() => useProfileViewModel());

    await waitFor(() => expect(result.current.status).toBe('error'));
    expect(result.current.error).toBe('no profile');
  });
});
