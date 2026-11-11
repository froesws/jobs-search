import { beforeEach, describe, expect, it, vi } from 'vitest';

const { openUrl } = vi.hoisted(() => ({ openUrl: vi.fn() }));

vi.mock('@tauri-apps/plugin-opener', () => ({ openUrl }));

import { AppError } from '@/lib/errors';
import { openExternalUrl } from '@/lib/ipc';

describe('openExternalUrl', () => {
  beforeEach(() => {
    openUrl.mockReset();
  });

  it('opens a valid http(s) URL once through the opener plugin', async () => {
    openUrl.mockResolvedValue(undefined);

    await openExternalUrl('https://example.com/jobs/1');

    expect(openUrl).toHaveBeenCalledTimes(1);
    expect(openUrl).toHaveBeenCalledWith('https://example.com/jobs/1');
  });

  it('normalizes the URL before opening it', async () => {
    openUrl.mockResolvedValue(undefined);

    await openExternalUrl('HTTPS://Example.com/jobs/1');

    expect(openUrl).toHaveBeenCalledWith('https://example.com/jobs/1');
  });

  it('rethrows a plugin rejection as an AppError', async () => {
    openUrl.mockRejectedValue('the OS refused to open the URL');

    await expect(openExternalUrl('https://example.com/jobs/1')).rejects.toBeInstanceOf(AppError);
    expect(openUrl).toHaveBeenCalledTimes(1);
  });
});
