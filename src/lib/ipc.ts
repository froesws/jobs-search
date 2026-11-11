import { invoke } from '@tauri-apps/api/core';

import { toAppError } from './errors';
import type {
  ActiveProvider,
  CandidateProfile,
  ExecutionAudit,
  JobApplication,
  JobDetail,
  JobEvaluationView,
  JobTab,
  PaginatedJobs,
  ResumeSummary,
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

export function saveProfile(profile: CandidateProfile): Promise<CandidateProfile> {
  return call('cmd_save_profile', { profile });
}

export function listResumes(): Promise<ResumeSummary[]> {
  return call('cmd_list_resumes');
}

export function uploadResume(filePath: string): Promise<CandidateProfile> {
  return call('cmd_upload_resume', { filePath });
}

export function deleteResume(fileId: string): Promise<CandidateProfile> {
  return call('cmd_delete_resume', { fileId });
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

export function applyJob(jobId: string): Promise<JobApplication> {
  return call('cmd_apply_job', { jobId });
}

export function ignoreJob(jobId: string): Promise<JobApplication> {
  return call('cmd_ignore_job', { jobId });
}

export function restoreJob(jobId: string): Promise<JobApplication> {
  return call('cmd_restore_job', { jobId });
}

export function approveJob(jobId: string): Promise<JobEvaluationView> {
  return call('cmd_approve_job', { jobId });
}

export function getAudit(runId: string): Promise<ExecutionAudit> {
  return call('cmd_get_audit', { runId });
}

export function getActiveProvider(): Promise<ActiveProvider> {
  return call('cmd_get_active_provider');
}
