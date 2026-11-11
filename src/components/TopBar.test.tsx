import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { getActiveProvider, listResumes } from '@/lib/ipc';
import { DEFAULT_FILTERS } from '@/lib/types';
import { providerFixture } from '@/test/fixtures';
import { beforeEach } from 'vitest';
import { FeedbackProvider } from '@/components/FeedbackProvider';
import { TopBar } from '@/components/TopBar';

vi.mock('@/lib/ipc', async () => {
  const { createIpcMock } = await import('@/test/ipc-mock');
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
    expect(screen.getByRole('group', { name: 'Seniority' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Trainee' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Junior' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Pleno' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Senior' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Any' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Intern' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Pleno-Senior' })).not.toBeInTheDocument();
    expect(screen.getByText('Profile default')).toBeInTheDocument();
    expect(screen.getByLabelText('Stack or language')).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Work modality' })).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Stack or language'), {
      target: { value: 'Rust' },
    });
    expect(onChange).toHaveBeenCalledWith({ stack_keyword: 'Rust' });

    fireEvent.click(screen.getByRole('button', { name: 'Refresh Search' }));
    expect(onSearch).toHaveBeenCalledTimes(1);
  });

  it('allows three seniority levels at once and removes a selected level', () => {
    const onChange = vi.fn();
    const { rerender } = render(
      <FeedbackProvider>
        <TopBar
          filters={{ ...DEFAULT_FILTERS, seniorities: ['Trainee', 'Junior'] }}
          onChange={onChange}
          onSearch={() => {}}
          searching={false}
          mode="dark"
          onToggleMode={() => {}}
        />
      </FeedbackProvider>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Pleno' }));
    expect(onChange).toHaveBeenLastCalledWith({
      seniorities: ['Trainee', 'Junior', 'Pleno'],
    });

    rerender(
      <FeedbackProvider>
        <TopBar
          filters={{ ...DEFAULT_FILTERS, seniorities: ['Trainee', 'Junior', 'Pleno'] }}
          onChange={onChange}
          onSearch={() => {}}
          searching={false}
          mode="dark"
          onToggleMode={() => {}}
        />
      </FeedbackProvider>,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Trainee' }));
    expect(onChange).toHaveBeenLastCalledWith({ seniorities: ['Junior', 'Pleno'] });
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
