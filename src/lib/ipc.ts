import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';

import { AppError, toAppError } from '@/lib/errors';
import { safeExternalUrl } from '@/lib/format';
import type {
  ActiveProvider,
  AnalyticsPayload,
  AnalyticsRequest,
  CandidateProfile,
  ExecutionAudit,
  ExportRequest,
  ExportResult,
  JobApplication,
  JobDetail,
  JobEvaluationView,
  JobTab,
  PaginatedJobs,
  ModelCatalogView,
  ProviderConfigInput,
  ProviderConfigView,
  ProviderTestResult,
  ProviderId,
  ProviderSettingField,
  ResumeSummary,
  SearchFilters,
  SearchMetrics,
  UploadResumeResult,
} from '@/lib/types';

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toAppError(error);
  }
}

/** Open an http(s) URL in the OS default browser. */
export async function openExternalUrl(value: string): Promise<void> {
  const url = safeExternalUrl(value);
  if (!url) {
    throw new AppError('INVALID_URL', 'The original URL is missing or invalid.');
  }
  try {
    await openUrl(url);
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

export function uploadResume(filePath: string): Promise<UploadResumeResult> {
  return call('cmd_upload_resume', { filePath });
}

export function deleteResume(fileId: string): Promise<CandidateProfile | null> {
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

export function getProviderConfig(): Promise<ProviderConfigView> {
  return call('cmd_get_provider_config');
}

export function saveProviderConfig(config: ProviderConfigInput): Promise<ProviderConfigView> {
  return call('cmd_save_provider_config', { config });
}

export function saveProviderCredential(
  provider: ProviderId,
  apiKey: string,
  allowEnvFallback = false,
): Promise<ProviderConfigView> {
  return call('cmd_save_provider_credential', { provider, apiKey, allowEnvFallback });
}

export function removeProviderCredential(
  provider: ProviderId,
  removeEnvEntry = false,
): Promise<ProviderConfigView> {
  return call('cmd_remove_provider_credential', { provider, removeEnvEntry });
}

export function setSessionSelection(
  provider: ProviderId,
  models: string[],
): Promise<ProviderConfigView> {
  return call('cmd_set_session_selection', { provider, models });
}

export function refreshProviderModels(
  provider: ProviderId,
  includePaid = false,
  refresh = true,
): Promise<ModelCatalogView> {
  return call('cmd_refresh_provider_models', { provider, includePaid, refresh });
}

export function testProvider(
  provider: ProviderId,
  draftApiKey?: string,
  draftBaseUrl?: string,
  model?: string,
): Promise<ProviderTestResult> {
  return call('cmd_test_provider', { provider, draftApiKey, draftBaseUrl, model });
}

export function resetProviderSetting(
  provider: ProviderId,
  field: ProviderSettingField,
): Promise<ProviderConfigView> {
  return call('cmd_reset_provider_setting', { provider, field });
}

export function getAnalytics(request: AnalyticsRequest): Promise<AnalyticsPayload> {
  return call('cmd_get_analytics', { request });
}

export function exportAnalyticsCsv(request: ExportRequest): Promise<ExportResult> {
  return call('cmd_export_analytics_csv', { request });
}
