import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import App from '@/App';
import { getActiveProvider, getJobs, getProfile, listResumes } from '@/lib/ipc';
import { providerFixture } from '@/test/fixtures';

vi.mock('@/lib/ipc', async () => {
  const { createIpcMock } = await import('@/test/ipc-mock');
  return createIpcMock();
});

const emptyPage = {
  items: [],
  total: 0,
  limit: 12,
  offset: 0,
  tab: 'NEW_JOBS' as const,
};

const profile = {
  profile_id: 'fixture',
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

describe('dashboard shell', () => {
  beforeEach(() => {
    vi.mocked(getJobs).mockResolvedValue(emptyPage);
    vi.mocked(getProfile).mockResolvedValue(profile);
    vi.mocked(listResumes).mockResolvedValue([]);
    vi.mocked(getActiveProvider).mockResolvedValue(providerFixture);
  });

  it('renders the four fixed tabs and the theme toggle', async () => {
    render(<App />);

    expect(screen.getByText('Jobs Search')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'New Jobs' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'My Applications' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'My Profile' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Analytics' })).toBeInTheDocument();

    const toggle = screen.getByRole('button', { name: 'Toggle color theme' });
    expect(toggle).toHaveTextContent('Light');
    fireEvent.click(toggle);
    expect(toggle).toHaveTextContent('Dark');

    await waitFor(() => expect(getJobs).toHaveBeenCalled());
  });

  it('exposes accessible navigation and controls', async () => {
    render(<App />);
    const tabs = screen.getAllByRole('tab');
    expect(tabs.map((tab) => tab.textContent)).toEqual([
      'New Jobs',
      'My Applications',
      'My Profile',
      'Analytics',
    ]);
    expect(screen.getByRole('button', { name: 'Toggle color theme' })).toBeInTheDocument();
    expect(screen.getByLabelText('Timeframe')).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Seniority' })).toBeInTheDocument();
    expect(screen.getByLabelText('Stack or language')).toBeInTheDocument();
    await waitFor(() => expect(getJobs).toHaveBeenCalled());
  });
});
