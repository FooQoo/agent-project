import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import '../../../shared/i18n';
import { CurrentVersionMenu } from './CurrentVersionMenu';

afterEach(cleanup);

function menu(current: string | null = 'v1.8.0') {
  const onSelect = vi.fn(async () => undefined);
  const onAdd = vi.fn(async () => undefined);
  render(
    <CurrentVersionMenu
      versions={['v1.8.0', 'v1.9.0']}
      current={current}
      onSelect={onSelect}
      onAdd={onAdd}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: /Current version/ }));
  return { onSelect, onAdd };
}

describe('the current version in the header', () => {
  it('shows the current version on its button and marks it in the list', () => {
    menu();
    expect(screen.getByRole('button', { name: 'Current version: v1.8.0' })).toBeTruthy();
    expect(screen.getByRole('radio', { name: /v1\.8\.0/ }).getAttribute('aria-checked')).toBe(
      'true',
    );
    expect(screen.getAllByText('Current').length).toBeGreaterThan(0);
  });

  it('makes another version current', async () => {
    const { onSelect } = menu();
    fireEvent.click(screen.getByRole('radio', { name: /v1\.9\.0/ }));
    await waitFor(() => expect(onSelect).toHaveBeenCalledWith('v1.9.0'));
  });

  it('clears the current version', async () => {
    const { onSelect } = menu();
    fireEvent.click(screen.getByRole('radio', { name: /No current version/ }));
    await waitFor(() => expect(onSelect).toHaveBeenCalledWith(null));
  });

  it('lists the versions newest first, whatever order they come in', () => {
    const onSelect = vi.fn(async () => undefined);
    render(
      <CurrentVersionMenu
        versions={['v1.9.0', 'v1.8.0', 'v1.10.0']}
        current={null}
        onSelect={onSelect}
        onAdd={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: /Current version/ }));
    const names = screen.getAllByRole('radio').map((radio) => radio.parentElement?.textContent);
    expect(names).toEqual(['v1.10.0', 'v1.9.0', 'v1.8.0', 'No current version']);
  });

  it('only lists the typed version: it does not become current', async () => {
    const { onAdd, onSelect } = menu(null);
    expect(screen.getByRole('button', { name: 'Current version: Not set' })).toBeTruthy();
    const confirm = screen.getByRole('button', {
      name: 'Add this version to the project',
    }) as HTMLButtonElement;
    expect(confirm.disabled).toBe(true);
    fireEvent.change(screen.getByLabelText('New version'), { target: { value: ' v2.0.0 ' } });
    fireEvent.click(confirm);
    await waitFor(() => expect(onAdd).toHaveBeenCalledWith('v2.0.0'));
    expect(onSelect).not.toHaveBeenCalled();
    // The list stays open and the field is empty again.
    await waitFor(() =>
      expect((screen.getByLabelText('New version') as HTMLInputElement).value).toBe(''),
    );
  });

  it('keeps the list open and says so when the change fails', async () => {
    const onSelect = vi.fn(async () => {
      throw new Error('no');
    });
    render(
      <CurrentVersionMenu
        versions={['v1.8.0']}
        current={null}
        onSelect={onSelect}
        onAdd={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: /Current version/ }));
    fireEvent.click(screen.getByRole('radio', { name: /v1\.8\.0/ }));
    expect((await screen.findByRole('alert')).textContent).toContain('Could not change');
  });
});
