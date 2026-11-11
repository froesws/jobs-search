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

export const DEFAULT_FILTERS: SearchFilters = {
  titles: [],
  stack_keyword: '',
  timeframe: 'D7',
  seniorities: [],
  modalities: [],
  source_ids: [],
};
