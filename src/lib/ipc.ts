import { invoke } from '@tauri-apps/api/core';

import { toAppError } from './errors';
import type {
  CandidateProfile,
  JobDetail,
  JobTab,
  PaginatedJobs,
  SearchFilters,
  SearchMetrics,
} from './types';

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toAppError(error);
  }
}

export function getProfile(): Promise<CandidateProfile> {
  return call('cmd_get_profile');
}

export function triggerSearch(filters: SearchFilters): Promise<SearchMetrics> {
  return call('cmd_trigger_search', { filters });
}

export function getJobs(tab: JobTab, limit: number, offset: number): Promise<PaginatedJobs> {
  return call('cmd_get_jobs', { tab, limit, offset });
}

export function getJob(jobId: string): Promise<JobDetail> {
  return call('cmd_get_job', { jobId });
}
