import type {
  ActiveProvider,
  AnalyticsPayload,
  CandidateProfile,
  ExecutionAudit,
  JobDetail,
  JobSummary,
  PaginatedJobs,
  SearchMetrics,
} from '@/lib/types';

export const emptyPage: PaginatedJobs = {
  items: [],
  total: 0,
  limit: 12,
  offset: 0,
  tab: 'NEW_JOBS',
};

export const profileFixture: CandidateProfile = {
  profile_id: 'fixture-profile-001',
  version: 1,
  updated_at: '2026-11-10T00:00:00Z',
  target_roles: ['Rust Engineer'],
  seniority_range: ['Trainee', 'Junior', 'Pleno'],
  work_modalities: ['Remote', 'Hybrid', 'Onsite'],
  accepted_cities: ['Porto Alegre', 'Curitiba'],
  acceptable_technologies: ['Rust', 'React'],
  core_competencies: ['Rust', 'React'],
  secondary_competencies: ['SQL'],
  domain_experience: ['systems programming'],
  dealbreakers: ['sales'],
  mandatory_criteria: ['location', 'modality'],
  work_authorization: 'Authorized to work in Brazil',
  salary_threshold: null,
  source_documents: [],
};

export const jobFixture: JobSummary = {
  job_id: 'fixture-job-1',
  title: 'Senior Rust Engineer',
  company: 'Acme',
  location: 'Remote - Brazil',
  work_modality: 'Remote',
  url: 'https://example.com/jobs/1',
  posted_at: '2026-11-08T12:00:00Z',
  evaluation_status: 'APPROVED',
  lifecycle_state: 'DISCOVERED',
  match_score: 92,
  matching_items_count: 4,
  total_requirements_count: 5,
  summary: 'Own the systems platform.',
  needs_review: false,
  applied_at: null,
};

export const appliedJobFixture: JobSummary = {
  ...jobFixture,
  job_id: 'fixture-job-applied',
  lifecycle_state: 'APPLIED',
  match_score: 88,
  matching_items_count: 3,
  total_requirements_count: 4,
  applied_at: '2026-11-10T12:00:00Z',
};

export const jobDetailFixture: JobDetail = {
  ...jobFixture,
  description_clean: 'Build reliable systems.',
  requirements: [
    {
      id: 'r1',
      text: 'Rust',
      dimension: 'CORE_TECHNICAL',
      match_type: 'EXACT',
      evidence: 'projects',
    },
    {
      id: 'r2',
      text: 'gRPC',
      dimension: 'CORE_TECHNICAL',
      match_type: 'NONE',
      evidence: '',
    },
  ],
  matched_keywords: ['Rust'],
  missing_keywords: ['gRPC'],
  hard_filter_rejections: [],
  source_lineage: [
    {
      source_id: 'src',
      source_name: 'jobspy',
      external_id: '1',
      url: 'https://example.com/jobs/1',
      ingested_at: '2026-11-09T00:00:00Z',
    },
  ],
  score_breakdown: null,
  provider: 'deepseek',
  model: 'deepseek-chat',
  error_category: null,
};

export const providerFixture: ActiveProvider = {
  provider: 'deepseek',
  model: 'deepseek-chat',
  fallback_enabled: false,
};

export const searchMetricsFixture: SearchMetrics = {
  run_id: 'run-1',
  status: 'PARTIAL',
  total_ingested: 5,
  duplicates_merged: 1,
  rejected_count: 1,
  pending_count: 0,
  approved_count: 1,
  error_count: 0,
  started_at: '2026-11-10T10:00:00Z',
  finished_at: '2026-11-10T10:01:00Z',
  source_failures: ['jobspy: timeout'],
};

export const auditFixture: ExecutionAudit = {
  run_id: 'run-1',
  status: 'PARTIAL',
  started_at: '2026-11-10T10:00:00Z',
  finished_at: '2026-11-10T10:05:00Z',
  search_config: { timeframe: 'D7' },
  total_ingested: 7,
  duplicates_merged: 2,
  rejected_count: 2,
  pending_count: 1,
  approved_count: 3,
  error_count: 1,
  applied_count: 2,
  ignored_count: 1,
  approval_rate: 50,
  application_rate: 66.7,
  rejected_by_reason: { SENIORITY_NOT_ELIGIBLE: 2 },
  source_failures: ['jobspy: timeout'],
  score_distribution: { '80-89': 2 },
  source_breakdown: { jobspy: 5 },
  provider_breakdown: { 'deepseek/deepseek-chat': 4 },
};

export const analyticsPayloadFixture: AnalyticsPayload = {
  scope: 'LAST_7_DAYS',
  run_id: null,
  denominator: 100,
  last_updated: '2026-11-10T00:00:00Z',
  stale: true,
  skill_gap: [
    {
      skill: 'kubernetes',
      display_name: 'Kubernetes',
      demand_count: 50,
      demand_percentage: 50,
      rank: 1,
    },
  ],
  funnel: {
    ingested: 100,
    duplicates: 0,
    rejected_by_reason: {},
    pending: 0,
    approved: 100,
    error: 0,
    applied: 0,
    ignored: 0,
    approval_rate: 100,
    application_rate: 0,
    source_failures: [],
    source_breakdown: {},
    provider_breakdown: {},
  },
  score_distribution: [{ range: '80-89', count: 100 }],
  llm_metrics: {
    call_count: 1,
    success_count: 1,
    error_count: 0,
    fallback_count: 0,
    p50_latency_ms: 100,
    p95_latency_ms: 100,
    by_provider: [],
  },
};
