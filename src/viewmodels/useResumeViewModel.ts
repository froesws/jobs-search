import { useCallback, useEffect, useState } from 'react';

import { safeMessage } from '@/lib/errors';
import { deleteResume, listResumes, uploadResume } from '@/lib/ipc';
import type { CandidateProfile, ResumeSummary, UploadResumeResult } from '@/lib/types';

type Status = 'loading' | 'ready' | 'empty' | 'error';

export function useResumeViewModel() {
  const [status, setStatus] = useState<Status>('loading');
  const [resumes, setResumes] = useState<ResumeSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    setStatus('loading');
    setError(null);
    try {
      const values = await listResumes();
      setResumes(values);
      setStatus(values.length === 0 ? 'empty' : 'ready');
    } catch (cause) {
      setStatus('error');
      setError(safeMessage(cause));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const upload = useCallback(
    async (filePath: string): Promise<UploadResumeResult> => {
      setBusy(true);
      try {
        const result = await uploadResume(filePath);
        await refresh();
        return result;
      } finally {
        setBusy(false);
      }
    },
    [refresh],
  );

  const remove = useCallback(
    async (fileId: string): Promise<CandidateProfile | null> => {
      setBusy(true);
      try {
        const profile = await deleteResume(fileId);
        await refresh();
        return profile;
      } finally {
        setBusy(false);
      }
    },
    [refresh],
  );

  return { status, resumes, error, busy, refresh, upload, remove };
}
