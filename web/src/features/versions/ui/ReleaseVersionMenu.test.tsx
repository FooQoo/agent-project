import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import '../../../shared/i18n';
import { ReleaseVersionMenu } from './ReleaseVersionMenu';

afterEach(cleanup);

function menu(filter: string | null = 'v1.8.0', release: string | null = 'v1.8.0') {
  const onFilter = vi.fn();
  const onMakeRelease = vi.fn(async () => undefined);
  const onAdd = vi.fn(async () => undefined);
  render(
    <ReleaseVersionMenu
      versions={['v1.8.0', 'v1.9.0']}
      release={release}
      filter={filter}
      onFilter={onFilter}
      onMakeRelease={onMakeRelease}
      onAdd={onAdd}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: /Version filter/ }));
  return { onFilter, onMakeRelease, onAdd };
}

describe('the version filter in the header', () => {
  it('shows the filter on its button and checks it in the list', () => {
    menu();
    expect(screen.getByRole('button', { name: 'Version filter: v1.8.0' })).toBeTruthy();
    expect(screen.getByRole('radio', { name: /v1\.8\.0/ }).getAttribute('aria-checked')).toBe(
      'true',
    );
  });

  it('marks the release version with a star', () => {
    menu('v1.9.0', 'v1.8.0');
    expect(screen.getByRole('radio', { name: /v1\.8\.0/ }).parentElement?.textContent).toContain(
      '\u2605',
    );
    expect(
      screen.getByRole('radio', { name: /v1\.9\.0/ }).parentElement?.textContent,
    ).not.toContain('\u2605');
  });

  it('filters by another version without changing the release version', () => {
    const { onFilter, onMakeRelease } = menu();
    fireEvent.click(screen.getByRole('radio', { name: /v1\.9\.0/ }));
    expect(onFilter).toHaveBeenCalledWith('v1.9.0');
    expect(onMakeRelease).not.toHaveBeenCalled();
  });

  it('clears the filter with All, which comes first and has no "No release version" item', () => {
    const { onFilter } = menu();
    expect(screen.queryByRole('radio', { name: /No release version/ })).toBeNull();
    expect(screen.getAllByRole('radio').map((radio) => radio.parentElement?.textContent)[0]).toBe(
      'All',
    );
    fireEvent.click(screen.getByRole('radio', { name: 'All' }));
    expect(onFilter).toHaveBeenCalledWith(null);
  });

  it('lists the versions newest first, whatever order they come in', () => {
    render(
      <ReleaseVersionMenu
        versions={['v1.9.0', 'v1.8.0', 'v1.10.0']}
        release={null}
        filter={null}
        onFilter={vi.fn()}
        onMakeRelease={vi.fn()}
        onAdd={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: /Version filter/ }));
    const names = screen.getAllByRole('radio').map((radio) => radio.parentElement?.textContent);
    expect(names).toEqual(['All', 'v1.10.0', 'v1.9.0', 'v1.8.0']);
  });

  it('makes the shown version the release version', async () => {
    const { onMakeRelease } = menu('v1.9.0', 'v1.8.0');
    fireEvent.click(
      screen.getByRole('button', { name: 'Make the shown v1.9.0 the release version' }),
    );
    await waitFor(() => expect(onMakeRelease).toHaveBeenCalledWith('v1.9.0'));
  });

  it('has no release button for All or for the version that already is the release version', () => {
    menu(null, 'v1.8.0');
    expect(screen.queryByRole('button', { name: /the release version/ })).toBeNull();
    cleanup();
    menu('v1.8.0', 'v1.8.0');
    expect(screen.queryByRole('button', { name: /the release version/ })).toBeNull();
  });

  it('only lists the typed version: nothing else changes', async () => {
    const { onAdd, onFilter, onMakeRelease } = menu(null, null);
    expect(screen.getByRole('button', { name: 'Version filter: All' })).toBeTruthy();
    const confirm = screen.getByRole('button', {
      name: 'Add this version to the project',
    }) as HTMLButtonElement;
    expect(confirm.disabled).toBe(true);
    fireEvent.change(screen.getByLabelText('New version'), { target: { value: ' v2.0.0 ' } });
    fireEvent.click(confirm);
    await waitFor(() => expect(onAdd).toHaveBeenCalledWith('v2.0.0'));
    expect(onFilter).not.toHaveBeenCalled();
    expect(onMakeRelease).not.toHaveBeenCalled();
    // The list stays open and the field is empty again.
    await waitFor(() =>
      expect((screen.getByLabelText('New version') as HTMLInputElement).value).toBe(''),
    );
  });

  it('keeps the list open and says so when making the release version fails', async () => {
    const onMakeRelease = vi.fn(async () => {
      throw new Error('no');
    });
    render(
      <ReleaseVersionMenu
        versions={['v1.8.0']}
        release={null}
        filter="v1.8.0"
        onFilter={vi.fn()}
        onMakeRelease={onMakeRelease}
        onAdd={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: /Version filter/ }));
    fireEvent.click(screen.getByRole('button', { name: /the release version/ }));
    expect((await screen.findByRole('alert')).textContent).toContain('Could not change');
  });
});
