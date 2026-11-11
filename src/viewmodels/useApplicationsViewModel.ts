import { useJobsViewModel } from './useJobsViewModel';

export function useApplicationsViewModel(refreshToken: number) {
  return useJobsViewModel('APPLICATIONS', refreshToken);
}
