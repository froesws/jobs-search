import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { getActiveProvider, listResumes } from '@/lib/ipc';
import { DEFAULT_FILTERS } from '@/lib/types';
import { providerFixture, providerStateFixture } from '@/test/fixtures';
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
          onOpenProviderSettings={() => {}}
          provider={providerStateFixture()}
          onSelectModel={() => {}}
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
    expect(screen.getAllByText('Profile default')).toHaveLength(2);
    expect(screen.getByLabelText('Stack or language')).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Work modality' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'On-site' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Hybrid' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Remote' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'All' })).not.toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Stack or language'), {
      target: { value: 'Rust' },
    });
    expect(onChange).toHaveBeenCalledWith({ stack_keyword: 'Rust' });

    fireEvent.click(screen.getByRole('button', { name: 'Refresh Search' }));
    expect(onSearch).toHaveBeenCalledTimes(1);
  });

  it('reserves the footer line when a filter is selected', () => {
    render(
      <FeedbackProvider>
        <TopBar
          filters={{ ...DEFAULT_FILTERS, seniorities: ['Trainee'] }}
          onChange={() => {}}
          onSearch={() => {}}
          searching={false}
          mode="dark"
          onToggleMode={() => {}}
          onOpenProviderSettings={() => {}}
          provider={providerStateFixture()}
          onSelectModel={() => {}}
        />
      </FeedbackProvider>,
    );

    const seniorityFooter = screen.getByTestId('seniority-footer');
    expect(seniorityFooter).toHaveAttribute('aria-hidden', 'true');
    expect(seniorityFooter).toHaveStyle({ visibility: 'hidden' });
    expect(screen.getByTestId('modality-footer')).toHaveStyle({
      visibility: 'hidden',
    });
    expect(screen.getByTestId('filter-row')).toHaveStyle({
      alignItems: 'flex-start',
    });
  });

  it('aligns seniority and work type selector tops with neighboring fields', () => {
    render(
      <FeedbackProvider>
        <TopBar
          filters={DEFAULT_FILTERS}
          onChange={() => {}}
          onSearch={() => {}}
          searching={false}
          mode="dark"
          onToggleMode={() => {}}
          onOpenProviderSettings={() => {}}
          provider={providerStateFixture()}
          onSelectModel={() => {}}
        />
      </FeedbackProvider>,
    );

    // The filter row wraps flex items starting at the same top edge.
    expect(screen.getByTestId('filter-row')).toHaveStyle({
      alignItems: 'flex-start',
    });

    // Selector wrappers stack caption above selector without extra top offset.
    expect(screen.getByRole('group', { name: 'Seniority' })).toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Work modality' })).toBeInTheDocument();

    // No filter control carries a margin-top that would push its selector below
    // its neighbors (Timeframe select / Stack field / seniority / work type).
    const controls = [
      screen.getByLabelText('Timeframe'),
      screen.getByRole('group', { name: 'Seniority' }),
      screen.getByLabelText('Stack or language'),
      screen.getByRole('group', { name: 'Work modality' }),
    ];
    for (const control of controls) {
      expect(control).toHaveStyle({ marginTop: '0px' });
    }
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
          onOpenProviderSettings={() => {}}
          provider={providerStateFixture()}
          onSelectModel={() => {}}
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
          onOpenProviderSettings={() => {}}
          provider={providerStateFixture()}
          onSelectModel={() => {}}
        />
      </FeedbackProvider>,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Trainee' }));
    expect(onChange).toHaveBeenLastCalledWith({ seniorities: ['Junior', 'Pleno'] });
  });

  it('allows Remote and Hybrid together and adds On-site', () => {
    const onChange = vi.fn();
    const { rerender } = render(
      <FeedbackProvider>
        <TopBar
          filters={{ ...DEFAULT_FILTERS, modalities: ['Remote'] }}
          onChange={onChange}
          onSearch={() => {}}
          searching={false}
          mode="dark"
          onToggleMode={() => {}}
          onOpenProviderSettings={() => {}}
          provider={providerStateFixture()}
          onSelectModel={() => {}}
        />
      </FeedbackProvider>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Hybrid' }));
    expect(onChange).toHaveBeenLastCalledWith({ modalities: ['Remote', 'Hybrid'] });

    rerender(
      <FeedbackProvider>
        <TopBar
          filters={{ ...DEFAULT_FILTERS, modalities: ['Hybrid', 'Remote'] }}
          onChange={onChange}
          onSearch={() => {}}
          searching={false}
          mode="dark"
          onToggleMode={() => {}}
          onOpenProviderSettings={() => {}}
          provider={providerStateFixture()}
          onSelectModel={() => {}}
        />
      </FeedbackProvider>,
    );
    fireEvent.click(screen.getByRole('button', { name: 'On-site' }));
    expect(onChange).toHaveBeenLastCalledWith({
      modalities: ['Hybrid', 'Remote', 'Onsite'],
    });
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
          onOpenProviderSettings={() => {}}
          provider={providerStateFixture()}
          onSelectModel={() => {}}
        />
      </FeedbackProvider>,
    );
    expect(screen.getByRole('button', { name: /Searching/ })).toBeDisabled();
  });
});
