export type JobTab = 'NEW_JOBS' | 'APPLICATIONS' | 'BELOW_THRESHOLD';

export type EvaluationStatus = 'PENDING' | 'APPROVED' | 'REJECTED' | 'ERROR';

export type LifecycleState = 'DISCOVERED' | 'APPLIED' | 'IGNORED';

export type WorkModality = 'Remote' | 'Hybrid' | 'Onsite' | 'Unknown';

export type Seniority = 'Trainee' | 'Intern' | 'Junior' | 'Pleno' | 'Senior';

export type SearchTimeframe = 'H24' | 'D7' | 'D30';

export interface SearchFilters {
  titles: string[];
  stack_keyword: string;
  timeframe: SearchTimeframe;
  seniorities: Seniority[];
  modalities: WorkModality[];
  source_ids: string[];
}

export interface JobSummary {
  job_id: string;
  title: string;
  company: string;
  location: string;
  work_modality: string;
  url: string;
  posted_at: string | null;
  evaluation_status: EvaluationStatus | null;
  lifecycle_state: LifecycleState;
  match_score: number | null;
  matching_items_count: number | null;
  total_requirements_count: number | null;
  summary: string;
  needs_review: boolean;
  applied_at: string | null;
}

export interface Requirement {
  id: string;
  text: string;
  dimension: 'CORE_TECHNICAL' | 'DOMAIN' | 'SENIORITY' | 'SECONDARY';
  match_type: 'EXACT' | 'INFERRED' | 'NONE';
  evidence: string;
}

export interface HardFilterRejection {
  code: string;
  message: string;
}

export interface SourceLineage {
  source_id: string;
  source_name: string;
  external_id: string;
  url: string;
  ingested_at: string;
}

export interface JobDetail extends JobSummary {
  description_clean: string;
  requirements: Requirement[];
  matched_keywords: string[];
  missing_keywords: string[];
  hard_filter_rejections: HardFilterRejection[];
  source_lineage: SourceLineage[];
  score_breakdown: Record<string, unknown> | null;
  provider: string | null;
  model: string | null;
  error_category: string | null;
}

export interface RunJobRow {
  jobId: string;
  title: string;
  company: string;
  location: string;
  workModality: string;
  url: string;
  evaluationStatus: string | null;
  matchScore: number | null;
  rejectionReason: string | null;
  provider: string | null;
  model: string | null;
}

export interface PaginatedRunJobs {
  items: RunJobRow[];
  total: number;
  limit: number;
  offset: number;
}

export interface PaginatedJobs {
  items: JobSummary[];
  total: number;
  limit: number;
  offset: number;
  tab: JobTab;
}

export interface CandidateProfile {
  profile_id: string;
  version: number;
  updated_at: string;
  target_roles: string[];
  seniority_range: string[];
  work_modalities: string[];
  accepted_cities: string[];
  acceptable_technologies: string[];
  core_competencies: string[];
  secondary_competencies: string[];
  domain_experience: string[];
  dealbreakers: string[];
  mandatory_criteria: string[];
  work_authorization: string | null;
  salary_threshold: number | null;
  source_documents: string[];
  extras?: Record<string, unknown>;
}

export interface SearchMetrics {
  run_id: string;
  status: string;
  total_ingested: number;
  duplicates_merged: number;
  rejected_count: number;
  pending_count: number;
  approved_count: number;
  error_count: number;
  started_at: string;
  finished_at: string | null;
  source_failures: string[];
}

export interface ResumeSummary {
  id: string;
  filename: string;
  content_hash: string;
  uploaded_at: string;
}

export interface UploadResumeResult {
  resume_id: string;
  profile: CandidateProfile | null;
  duplicate: boolean;
  warning: string | null;
}

export interface ActiveProvider {
  provider: string;
  model: string;
  fallback_enabled: boolean;
  session_only?: boolean;
}

export type ProviderId = 'gemini' | 'openrouter' | 'deepseek' | 'ollama';

export type CredentialSource = 'credential_store' | 'env' | 'none';

export interface ProviderSettingView {
  id: ProviderId;
  label: string;
  requiresApiKey: boolean;
  hasCredential: boolean;
  credentialSource: CredentialSource;
  model: string | null;
  models: string[] | null;
  baseUrl: string | null;
  attempts: number;
  enabled: boolean;
}

export interface SessionSelection {
  provider: ProviderId;
  models: string[];
}

export interface ProviderConfigView {
  providers: ProviderSettingView[];
  order: ProviderId[];
  fallbackEnabled: boolean;
  sessionOverride: SessionSelection | null;
}

export interface ProviderSettingInput {
  id: ProviderId;
  model?: string;
  models?: string[];
  baseUrl?: string;
  attempts?: number;
  enabled?: boolean;
}

export interface ProviderConfigInput {
  order: ProviderId[];
  fallbackEnabled: boolean;
  providers: ProviderSettingInput[];
}

export interface ModelOption {
  id: string;
  price: string | null;
}

export interface ModelCatalogView {
  provider: ProviderId;
  models: ModelOption[];
  source: 'cache' | 'builtin';
  fetchedAt: string | null;
  degraded: boolean;
  message: string | null;
}

export interface ProviderTestResult {
  ok: boolean;
  latencyMs: number;
  message: string;
}

export type ProviderSettingField =
  'model' | 'models' | 'baseUrl' | 'attempts' | 'enabled' | 'order' | 'fallbackEnabled';

export interface JobApplication {
  job_id: string;
  lifecycle_state: LifecycleState;
  applied_at: string | null;
  approval_source: 'automatic' | 'manual';
  evaluation_snapshot_json: string;
}

export interface JobEvaluationView {
  job_id: string;
  evaluation_status: EvaluationStatus;
  match_score: number | null;
  approval_source: 'automatic' | 'manual';
}

export interface ExecutionAudit {
  run_id: string;
  status: string;
  started_at: string;
  finished_at: string | null;
  search_config: Record<string, unknown>;
  total_ingested: number;
  duplicates_merged: number;
  rejected_count: number;
  pending_count: number;
  approved_count: number;
  error_count: number;
  applied_count: number;
  ignored_count: number;
  approval_rate: number;
  application_rate: number;
  rejected_by_reason: Record<string, number>;
  source_failures: string[];
  score_distribution: Record<string, number>;
  source_breakdown: Record<string, number>;
  provider_breakdown: Record<string, number>;
}

export type AnalyticsScope = 'CURRENT_RUN' | 'LAST_7_DAYS';

export interface AnalyticsRequest {
  scope: AnalyticsScope;
  run_id?: string | null;
}

export interface SkillGapMetric {
  skill: string;
  display_name: string;
  demand_count: number;
  demand_percentage: number;
  rank: number;
}

export interface FunnelPayload {
  ingested: number;
  duplicates: number;
  rejected_by_reason: Record<string, number>;
  pending: number;
  approved: number;
  error: number;
  applied: number;
  ignored: number;
  approval_rate: number;
  application_rate: number;
  source_failures: string[];
  source_breakdown: Record<string, number>;
  provider_breakdown: Record<string, number>;
}

export interface ScoreBucket {
  range: string;
  count: number;
}

export interface ProviderMetric {
  provider: string;
  model: string;
  call_count: number;
  success_count: number;
  error_count: number;
  fallback_count: number;
  p50_latency_ms: number;
  p95_latency_ms: number;
}

export interface LlmMetricsPayload {
  call_count: number;
  success_count: number;
  error_count: number;
  fallback_count: number;
  p50_latency_ms: number;
  p95_latency_ms: number;
  by_provider: ProviderMetric[];
}

export interface AnalyticsPayload {
  scope: AnalyticsScope;
  run_id: string | null;
  denominator: number;
  last_updated: string;
  stale: boolean;
  skill_gap: SkillGapMetric[];
  funnel: FunnelPayload;
  score_distribution: ScoreBucket[];
  llm_metrics: LlmMetricsPayload;
}

export interface ExportRequest {
  scope: AnalyticsScope;
  run_id?: string | null;
  destination: string;
}

export interface ExportResult {
  path: string;
  rows: number;
  bytes: number;
}

export const DEFAULT_FILTERS: SearchFilters = {
  titles: [],
  stack_keyword: '',
  timeframe: 'D7',
  seniorities: [],
  modalities: [],
  source_ids: [],
};
