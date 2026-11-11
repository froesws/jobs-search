import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { open } from '@tauri-apps/plugin-dialog';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { deleteResume, listResumes, uploadResume } from '../lib/ipc';
import type { CandidateProfile } from '../lib/types';
import { FeedbackProvider } from '../components/FeedbackProvider';
import { ResumeManager } from '../components/ResumeManager';

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}));

vi.mock('../lib/ipc', () => ({
  listResumes: vi.fn(),
  uploadResume: vi.fn(),
  deleteResume: vi.fn(),
}));

const profile = {
  profile_id: 'p1',
  extras: { resumes_available: true },
} as unknown as CandidateProfile;

describe('ResumeManager', () => {
  beforeEach(() => {
    vi.mocked(listResumes).mockResolvedValue([
      {
        id: 'r1',
        filename: 'resume.pdf',
        content_hash: 'sha',
        uploaded_at: '2026-11-10T00:00:00Z',
      },
    ]);
    vi.mocked(open).mockResolvedValue('/tmp/resume.pdf');
    vi.mocked(uploadResume).mockResolvedValue(profile);
    vi.mocked(deleteResume).mockResolvedValue(profile);
  });

  it('uploads a selected PDF and refreshes the list', async () => {
    render(
      <FeedbackProvider>
        <ResumeManager />
      </FeedbackProvider>,
    );
    await waitFor(() => expect(screen.getByText('Resumes (1)')).toBeInTheDocument());

    fireEvent.click(screen.getByRole('button', { name: 'Upload resume' }));
    await waitFor(() => expect(uploadResume).toHaveBeenCalledWith('/tmp/resume.pdf'));
    expect(await screen.findByText(/Resume processed/)).toBeInTheDocument();
  });

  it('deletes a stored resume from the menu', async () => {
    render(
      <FeedbackProvider>
        <ResumeManager />
      </FeedbackProvider>,
    );
    await waitFor(() => expect(screen.getByText('Resumes (1)')).toBeInTheDocument());

    fireEvent.click(screen.getByRole('button', { name: 'Resumes (1)' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Delete' }));

    await waitFor(() => expect(deleteResume).toHaveBeenCalledWith('r1'));
  });
});
