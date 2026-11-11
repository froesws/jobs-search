import { useJobsViewModel } from '@/viewmodels/useJobsViewModel';

export function useApplicationsViewModel(refreshToken: number) {
  return useJobsViewModel('APPLICATIONS', refreshToken);
}
