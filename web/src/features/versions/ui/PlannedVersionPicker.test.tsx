import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import '../../../shared/i18n';
import type { PendingSave } from '../../../shared/ui/editor/planField';
import { PlannedVersionChangedError } from '../api/versions';
import { PlannedVersionPicker } from './PlannedVersionPicker';

afterEach(cleanup);

/** Renders the picker and returns the pending change the editor would run when it saves. */
function picker(props: Partial<Parameters<typeof PlannedVersionPicker>[0]> = {}) {
  let pending: PendingSave | null = null;
  const onPending = vi.fn((next: PendingSave | null) => {
    pending = next;
  });
  const onSave = vi.fn(async () => undefined);
  render(
    <PlannedVersionPicker
      taskId={7}
      planned={null}
      current="v1.8.0"
      versions={['v1.8.0', 'v1.9.0']}
      onSave={onSave}
      onPending={onPending}
      {...props}
    />,
  );
  return { onSave, onPending, pending: () => pending };
}

describe('the planned version field of the task editor', () => {
  it('offers current, the other versions and none', () => {
    picker();
    expect(screen.getByRole('radio', { name: /Make it current \(v1\.8\.0\)/ })).toBeTruthy();
    expect(screen.getByRole('radio', { name: /v1\.9\.0/ })).toBeTruthy();
    expect(screen.getByRole('radio', { name: /Leave unplanned/ })).toBeTruthy();
  });

  it('disables current while the project has none', () => {
    picker({ current: null });
    const current = screen.getByRole('radio', { name: /Make it current/ }) as HTMLButtonElement;
    expect(current.disabled).toBe(true);
    expect(screen.getByText('Unavailable: no current version is set')).toBeTruthy();
  });

  it('has nothing to save until the person changes the choice', () => {
    const { onPending, pending } = picker({ planned: 'v1.9.0' });
    expect(onPending).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('radio', { name: /Leave unplanned/ }));
    expect(pending()).not.toBeNull();
    // Choosing the saved value again takes the change back.
    fireEvent.click(screen.getByRole('radio', { name: /v1\.9\.0/ }));
    expect(pending()).toBeNull();
  });

  it('saves over the value it shows, with the choice made', async () => {
    const { onSave, pending } = picker({ planned: 'v1.9.0' });
    fireEvent.click(screen.getByRole('radio', { name: /Make it current/ }));
    await pending()?.();
    expect(onSave).toHaveBeenCalledWith({ choice: 'current' }, 'v1.9.0');
    fireEvent.click(screen.getByRole('radio', { name: /Leave unplanned/ }));
    await pending()?.();
    expect(onSave).toHaveBeenLastCalledWith({ choice: 'unset' }, 'v1.9.0');
  });

  it('sends a named version', async () => {
    const { onSave, pending } = picker();
    fireEvent.click(screen.getByRole('radio', { name: /v1\.9\.0/ }));
    await pending()?.();
    expect(onSave).toHaveBeenCalledWith({ choice: 'named', version: 'v1.9.0' }, null);
  });

  it('reports a conflict with the newer value, reads the board again and rejects the save', async () => {
    const onSave = vi.fn(async () => {
      throw new PlannedVersionChangedError('changed', 'v1.9.0');
    });
    const onRefetch = vi.fn();
    const { pending } = picker({ onSave, onRefetch });
    fireEvent.click(screen.getByRole('radio', { name: /Make it current/ }));
    await expect(pending()?.()).rejects.toBeInstanceOf(PlannedVersionChangedError);
    const alert = await screen.findByRole('alert');
    expect(alert.textContent).toContain('v1.9.0');
    expect(alert.textContent).toContain('was not saved');
    expect(onRefetch).toHaveBeenCalledTimes(1);
    // Nothing was written again: the person has to decide and save once more.
    expect(onSave).toHaveBeenCalledTimes(1);
  });

  it('saves over the newer value only after the person saves again', async () => {
    const onSave = vi
      .fn<(choice: unknown, expected: string | null) => Promise<void>>()
      .mockRejectedValueOnce(new PlannedVersionChangedError('changed', 'v1.9.0'))
      .mockResolvedValue(undefined);
    const { pending } = picker({ onSave });
    fireEvent.click(screen.getByRole('radio', { name: /Make it current/ }));
    await expect(pending()?.()).rejects.toBeInstanceOf(PlannedVersionChangedError);
    expect(onSave).toHaveBeenLastCalledWith({ choice: 'current' }, null);
    // The message named v1.9.0: saving again is a deliberate replacement of it.
    await pending()?.();
    expect(onSave).toHaveBeenLastCalledWith({ choice: 'current' }, 'v1.9.0');
  });

  it('does not renew the expected value when the board updates by itself', async () => {
    const { onSave, pending } = picker();
    fireEvent.click(screen.getByRole('radio', { name: /v1\.9\.0/ }));
    // Another change planned the task meanwhile and the board showed it: the choice still saves over
    // what the person saw, so the server refuses it instead of replacing the newer value silently.
    await pending()?.();
    expect(onSave).toHaveBeenLastCalledWith({ choice: 'named', version: 'v1.9.0' }, null);
  });

  it('reports a failed save', async () => {
    const { pending } = picker({
      onSave: vi.fn(async () => {
        throw new Error('network');
      }),
    });
    fireEvent.click(screen.getByRole('radio', { name: /v1\.9\.0/ }));
    await expect(pending()?.()).rejects.toThrow('network');
    expect((await screen.findByRole('alert')).textContent).toContain('Could not save');
  });

  it('keeps versions named like the controls apart from the controls', async () => {
    const { onSave, pending } = picker({
      versions: ['v1.8.0', '__none__', 'c:current'],
      planned: '__none__',
    });
    // The task is planned for the version named "__none__", not unplanned.
    expect(screen.getByRole('radio', { name: '__none__' }).getAttribute('aria-checked')).toBe(
      'true',
    );
    fireEvent.click(screen.getByRole('radio', { name: 'c:current' }));
    await pending()?.();
    expect(onSave).toHaveBeenLastCalledWith({ choice: 'named', version: 'c:current' }, '__none__');
  });
});
