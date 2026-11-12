import Box from '@mui/material/Box';
import { useState } from 'react';

import { safeMessage } from '@/lib/errors';
import { normalizeFilters, validateFilters } from '@/lib/filters';
import {
  applyJob,
  approveJob,
  ignoreJob,
  restoreJob,
  setSessionSelection,
  triggerSearch,
} from '@/lib/ipc';
import type { JobSummary, ProviderId, SearchMetrics } from '@/lib/types';
import { JobDrawer } from '@/components/JobDrawer';
import { useFeedback } from '@/components/FeedbackProvider';
import { TabBar } from '@/components/TabBar';
import { TopBar } from '@/components/TopBar';
import { ProviderConfigDialog } from '@/components/ProviderConfigDialog';
import { useDashboardState } from '@/viewmodels/useDashboardState';
import { useJobDetailViewModel } from '@/viewmodels/useJobDetailViewModel';
import { useProviderConfig } from '@/viewmodels/useProviderConfig';
import { useSearchProgress } from '@/viewmodels/useSearchProgress';
import { ApplicationsView } from '@/views/ApplicationsView';
import { NewJobsView } from '@/views/NewJobsView';
import { ProfileView } from '@/views/ProfileView';
import { AnalyticsView } from '@/views/AnalyticsView';
import type { ThemeMode } from '@/theme/theme';

export function DashboardView({
  mode,
  onToggleMode,
}: {
  mode: ThemeMode;
  onToggleMode: () => void;
}) {
  const dashboard = useDashboardState();
  const detail = useJobDetailViewModel();
  const providerConfig = useProviderConfig();
  const [providerOpen, setProviderOpen] = useState(false);
  const feedback = useFeedback();
  const [refreshToken, setRefreshToken] = useState(0);
  const [searching, setSearching] = useState(false);
  const [actionBusy, setActionBusy] = useState(false);
  const [metrics, setMetrics] = useState<SearchMetrics | null>(null);
  const progress = useSearchProgress(searching);

  const handleSearch = async () => {
    const invalid = validateFilters(dashboard.filters);
    if (invalid) {
      feedback.notify(invalid, 'warning');
      return;
    }
    setSearching(true);
    try {
      const result = await triggerSearch(normalizeFilters(dashboard.filters));
      setMetrics(result);
      setRefreshToken((value) => value + 1);
      feedback.notify(
        result.status === 'FAILED'
          ? 'Search failed. Check the source failures and provider configuration.'
          : `Search completed: ${result.approved_count} approved, ${result.pending_count} pending.`,
        result.status === 'FAILED'
          ? 'error'
          : result.source_failures.length
            ? 'warning'
            : 'success',
        result.status === 'FAILED',
      );
    } catch (error) {
      feedback.notify(safeMessage(error), 'error', true);
    } finally {
      setSearching(false);
    }
  };

  const handleSelectModel = (providerId: ProviderId, models: string[]) => {
    void setSessionSelection(providerId, models)
      .then(() => providerConfig.reload())
      .catch((cause) => feedback.notify(safeMessage(cause), 'error', true));
  };

  const handleOpenJob = (job: JobSummary) => {
    dashboard.openJob(job.job_id);
    detail.open(job.job_id);
  };

  const runAction = async (action: () => Promise<unknown>, success: string) => {
    setActionBusy(true);
    try {
      await action();
      feedback.notify(success, 'success');
      detail.close();
      dashboard.closeJob();
      setRefreshToken((value) => value + 1);
    } catch (error) {
      feedback.notify(safeMessage(error), 'error', true);
    } finally {
      setActionBusy(false);
    }
  };

  const handleApply = (jobId: string) =>
    void runAction(
      () => applyJob(jobId),
      'Job marked as applied. The evaluation snapshot is preserved.',
    );
  const handleIgnore = (jobId: string) =>
    void runAction(() => ignoreJob(jobId), 'Job ignored and removed from New Jobs.');
  const handleRestore = (jobId: string) =>
    void runAction(() => restoreJob(jobId), 'Job restored to New Jobs.');
  const handleApprove = (jobId: string) =>
    void runAction(
      () => approveJob(jobId),
      'Job approved manually. The original score and breakdown were preserved.',
    );

  return (
    <Box sx={{ minHeight: '100vh', display: 'flex', flexDirection: 'column' }}>
      <TopBar
        filters={dashboard.filters}
        onChange={dashboard.setFilters}
        onSearch={() => void handleSearch()}
        searching={searching}
        mode={mode}
        onToggleMode={onToggleMode}
        onOpenProviderSettings={() => setProviderOpen(true)}
        provider={providerConfig}
        onSelectModel={handleSelectModel}
      />
      <TabBar value={dashboard.tab} onChange={dashboard.setTab} />

      <Box
        component="main"
        sx={{
          width: '100%',
          maxWidth: 1520,
          mx: 'auto',
          px: 3,
          py: 3,
          flex: 1,
        }}
      >
        {dashboard.tab === 'NEW_JOBS' ? (
          <NewJobsView
            refreshToken={refreshToken}
            metrics={metrics}
            onSearch={() => void handleSearch()}
            onOpenJob={handleOpenJob}
            searching={searching}
            progress={progress}
          />
        ) : null}
        {dashboard.tab === 'APPLICATIONS' ? (
          <ApplicationsView refreshToken={refreshToken} onOpenJob={handleOpenJob} />
        ) : null}
        {dashboard.tab === 'PROFILE' ? <ProfileView /> : null}
        {dashboard.tab === 'ANALYTICS' ? <AnalyticsView /> : null}
      </Box>

      <ProviderConfigDialog
        open={providerOpen}
        onClose={() => setProviderOpen(false)}
        state={providerConfig}
      />

      <JobDrawer
        open={detail.isDrawerOpen}
        job={detail.job}
        status={detail.status}
        error={detail.error}
        busy={actionBusy}
        onClose={() => {
          detail.close();
          dashboard.closeJob();
        }}
        onRetry={detail.retry}
        onApply={handleApply}
        onIgnore={handleIgnore}
        onRestore={handleRestore}
        onApprove={handleApprove}
      />
    </Box>
  );
}
