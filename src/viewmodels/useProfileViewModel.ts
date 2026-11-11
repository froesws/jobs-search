import { useCallback, useEffect, useState } from 'react';

import { safeMessage } from '../lib/errors';
import { getProfile, saveProfile } from '../lib/ipc';
import type { CandidateProfile } from '../lib/types';

type Status = 'idle' | 'loading' | 'ready' | 'empty' | 'error';

export function useProfileViewModel() {
  const [status, setStatus] = useState<Status>('idle');
  const [profile, setProfile] = useState<CandidateProfile | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const refresh = useCallback(async () => {
    setStatus('loading');
    setError(null);
    try {
      const value = await getProfile();
      setProfile(value);
      setStatus('ready');
    } catch (cause) {
      setStatus('error');
      setError(safeMessage(cause));
    }
  }, []);

  const save = useCallback(async (next: CandidateProfile): Promise<CandidateProfile> => {
    setSaving(true);
    try {
      const saved = await saveProfile(next);
      setProfile(saved);
      setStatus('ready');
      return saved;
    } finally {
      setSaving(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { status, profile, error, saving, refresh, save };
}
