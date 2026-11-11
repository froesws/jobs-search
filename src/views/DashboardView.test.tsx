import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import App from '../App';
import { applyJob, getActiveProvider, getJob, getJobs, getProfile, listResumes } from '../lib/ipc';
import type { JobDetail, JobSummary } from '../lib/types';

vi.mock('../lib/ipc', () => ({
  getJobs: vi.fn(),
  getProfile: vi.fn(),
  getJob: vi.fn(),
  triggerSearch: vi.fn(),
  listResumes: vi.fn().mockResolvedValue([]),
  uploadResume: vi.fn(),
  deleteResume: vi.fn(),
  saveProfile: vi.fn(),
  applyJob: vi.fn(),
  ignoreJob: vi.fn(),
  restoreJob: vi.fn(),
  approveJob: vi.fn(),
  getAudit: vi.fn(),
  getActiveProvider: vi.fn(),
}));

const summary: JobSummary = {
  job_id: 'j1',
  title: 'Rust Engineer',
  company: 'Acme',
  location: 'Remote - Brazil',
  work_modality: 'Remote',
  url: 'https://example.com/1',
  posted_at: '2026-11-08T00:00:00Z',
  evaluation_status: 'APPROVED',
  lifecycle_state: 'DISCOVERED',
  match_score: 91,
  matching_items_count: 4,
  total_requirements_count: 5,
  summary: 'Systems role',
  needs_review: false,
  applied_at: null,
};

const detail: JobDetail = {
  ...summary,
  description_clean: 'Rust and Linux',
  requirements: [],
  matched_keywords: ['Rust'],
  missing_keywords: [],
  hard_filter_rejections: [],
  source_lineage: [],
  score_breakdown: null,
  provider: 'deepseek',
  model: 'deepseek-chat',
  error_category: null,
};

const emptyPage = {
  items: [],
  total: 0,
  limit: 12,
  offset: 0,
  tab: 'NEW_JOBS' as const,
};

describe('dashboard application flow', () => {
  beforeEach(() => {
    vi.mocked(getActiveProvider).mockResolvedValue({
      provider: 'deepseek',
      model: 'deepseek-chat',
      fallback_enabled: false,
    });
    vi.mocked(listResumes).mockResolvedValue([]);
    vi.mocked(getProfile).mockResolvedValue({
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
    });
    vi.mocked(getJob).mockResolvedValue(detail);
    vi.mocked(applyJob).mockResolvedValue({
      job_id: 'j1',
      lifecycle_state: 'APPLIED',
      applied_at: '2026-11-10T12:00:00Z',
      approval_source: 'automatic',
      evaluation_snapshot_json: '{}',
    });
  });

  it('applies a job, closes the drawer, and refreshes it out of New Jobs', async () => {
    vi.mocked(getJobs)
      .mockResolvedValueOnce({
        items: [summary],
        total: 1,
        limit: 12,
        offset: 0,
        tab: 'NEW_JOBS',
      })
      .mockResolvedValue(emptyPage);

    render(<App />);
    fireEvent.click(await screen.findByText('Rust Engineer'));
    fireEvent.click(await screen.findByRole('button', { name: 'I Applied' }));

    await waitFor(() => expect(applyJob).toHaveBeenCalledWith('j1'));
    expect(await screen.findByText('No approved jobs yet')).toBeInTheDocument();
  });
});
