import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import App from './App';
import { getJobs, getProfile } from './lib/ipc';

vi.mock('./lib/ipc', () => ({
  getJobs: vi.fn(),
  getProfile: vi.fn(),
  getJob: vi.fn(),
  triggerSearch: vi.fn(),
}));

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
});
