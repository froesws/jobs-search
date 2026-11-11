import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getActiveProvider } from '../lib/ipc';
import { ProviderIndicator } from './ProviderIndicator';

vi.mock('../lib/ipc', () => ({
  getActiveProvider: vi.fn(),
}));

describe('ProviderIndicator', () => {
  beforeEach(() => {
    vi.mocked(getActiveProvider).mockReset();
  });

  it('shows the active provider and model without credentials', async () => {
    vi.mocked(getActiveProvider).mockResolvedValue({
      provider: 'deepseek',
      model: 'deepseek-chat',
      fallback_enabled: false,
    });
    render(<ProviderIndicator />);

    expect(await screen.findByText('deepseek · deepseek-chat')).toBeInTheDocument();
    expect(screen.queryByText(/secret|key/i)).not.toBeInTheDocument();
    await waitFor(() => expect(getActiveProvider).toHaveBeenCalledTimes(1));
  });
});
