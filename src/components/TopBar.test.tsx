import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { getActiveProvider, listResumes } from '../lib/ipc';
import { DEFAULT_FILTERS } from '../lib/types';
import { providerFixture } from '../test/fixtures';
import { beforeEach } from 'vitest';
import { FeedbackProvider } from './FeedbackProvider';
import { TopBar } from './TopBar';

vi.mock('../lib/ipc', async () => {
  const { createIpcMock } = await import('../test/ipc-mock');
  return createIpcMock();
});

describe('TopBar', () => {
  beforeEach(() => {
    vi.mocked(listResumes).mockResolvedValue([]);
    vi.mocked(getActiveProvider).mockResolvedValue(providerFixture);
  });

  it('renders the fixed search controls and triggers a run', () => {
    const onChange = vi.fn();
    const onSearch = vi.fn();
    render(
      <FeedbackProvider>
        <TopBar
          filters={DEFAULT_FILTERS}
          onChange={onChange}
          onSearch={onSearch}
          searching={false}
          mode="dark"
          onToggleMode={() => {}}
        />
      </FeedbackProvider>,
    );

    expect(screen.getByRole('button', { name: 'Refresh Search' })).toBeInTheDocument();
    expect(screen.getByLabelText('Timeframe')).toBeInTheDocument();
    expect(screen.getByLabelText('Seniority')).toBeInTheDocument();
    expect(screen.getByLabelText('Stack or language')).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Work modality' })).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Stack or language'), {
      target: { value: 'Rust' },
    });
    expect(onChange).toHaveBeenCalledWith({ stack_keyword: 'Rust' });

    fireEvent.click(screen.getByRole('button', { name: 'Refresh Search' }));
    expect(onSearch).toHaveBeenCalledTimes(1);
  });

  it('disables the search trigger while a run is active', () => {
    render(
      <FeedbackProvider>
        <TopBar
          filters={DEFAULT_FILTERS}
          onChange={() => {}}
          onSearch={() => {}}
          searching
          mode="dark"
          onToggleMode={() => {}}
        />
      </FeedbackProvider>,
    );
    expect(screen.getByRole('button', { name: /Searching/ })).toBeDisabled();
  });
});
