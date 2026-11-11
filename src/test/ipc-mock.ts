import { vi } from 'vitest';
import type { Mock } from 'vitest';

export interface IpcMock {
  getProfile: Mock;
  saveProfile: Mock;
  listResumes: Mock;
  uploadResume: Mock;
  deleteResume: Mock;
  triggerSearch: Mock;
  getJobs: Mock;
  getJob: Mock;
  applyJob: Mock;
  ignoreJob: Mock;
  restoreJob: Mock;
  approveJob: Mock;
  getAudit: Mock;
  getActiveProvider: Mock;
  getAnalytics: Mock;
  exportAnalyticsCsv: Mock;
  openExternalUrl: Mock;
}

/** Deterministic IPC doubles for component and ViewModel tests. */
export function createIpcMock(): IpcMock {
  return {
    getProfile: vi.fn(),
    saveProfile: vi.fn(),
    listResumes: vi.fn(),
    uploadResume: vi.fn(),
    deleteResume: vi.fn(),
    triggerSearch: vi.fn(),
    getJobs: vi.fn(),
    getJob: vi.fn(),
    applyJob: vi.fn(),
    ignoreJob: vi.fn(),
    restoreJob: vi.fn(),
    approveJob: vi.fn(),
    getAudit: vi.fn(),
    getActiveProvider: vi.fn(),
    getAnalytics: vi.fn(),
    exportAnalyticsCsv: vi.fn(),
    openExternalUrl: vi.fn(),
  };
}
