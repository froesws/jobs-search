import type { CandidateProfile, JobDetail, JobSummary, PaginatedJobs } from '../lib/types';

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

export const jobDetailFixture: JobDetail = {
  ...jobFixture,
  description_clean: 'Build reliable systems.',
  requirements: [],
  matched_keywords: ['Rust'],
  missing_keywords: ['gRPC'],
  hard_filter_rejections: [],
  source_lineage: [],
  score_breakdown: null,
  provider: 'deepseek',
  model: 'deepseek-chat',
  error_category: null,
};
