import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import '../../../shared/i18n';
import type { PendingSave } from '../../../shared/ui/editor/planField';
import type { EpicVersionResult } from '../api/versions';
import { EpicPlanField } from './EpicPlanField';

afterEach(cleanup);

const result: EpicVersionResult = {
  changed: [1],
  unchanged: [],
  skipped: [],
  skippedReasons: {},
  dryRun: false,
};

function field(current: string | null = 'v1.8.0', versions = ['v1.8.0', 'v1.9.0']) {
  let pending: PendingSave | null = null;
  const onPending = vi.fn((next: PendingSave | null) => {
    pending = next;
  });
  const apply = vi.fn(async () => result);
  render(
    <EpicPlanField current={current} versions={versions} apply={apply} onPending={onPending} />,
  );
  return { apply, pending: () => pending as PendingSave | null };
}

describe('the planned version of an epic, in its editor', () => {
  it('has no change to save by default', () => {
    const { pending } = field();
    expect(pending()).toBeNull();
    expect(screen.getByRole('radio', { name: 'No change' }).getAttribute('aria-checked')).toBe(
      'true',
    );
  });

  it('applies the chosen version when the editor saves, with nothing asked before', async () => {
    const { apply, pending } = field();
    fireEvent.click(screen.getByRole('radio', { name: 'v1.9.0' }));
    await waitFor(() => expect(pending()).not.toBeNull());
    expect(apply).not.toHaveBeenCalled();
    await pending()?.();
    expect(apply).toHaveBeenCalledWith({ choice: 'named', version: 'v1.9.0' });
  });

  it('offers current, none, and the versions newest first', async () => {
    const { apply, pending } = field('v1.8.0', ['v1.8.0', 'v1.9.0', 'v1.10.0']);
    const names = screen.getAllByRole('radio').map((r) => r.parentElement?.textContent);
    expect(names[0]).toBe('No change');
    expect(names.slice(-3)).toEqual(['v1.10.0', 'v1.9.0', 'Leave unplanned']);
    fireEvent.click(screen.getByRole('radio', { name: /Make it current/ }));
    await waitFor(() => expect(pending()).not.toBeNull());
    await pending()?.();
    expect(apply).toHaveBeenLastCalledWith({ choice: 'current' });
    fireEvent.click(screen.getByRole('radio', { name: 'Leave unplanned' }));
    await waitFor(() => expect(pending()).not.toBeNull());
    await pending()?.();
    expect(apply).toHaveBeenLastCalledWith({ choice: 'unset' });
  });

  it('disables current while none is set', () => {
    field(null);
    expect(
      (screen.getByRole('radio', { name: /Make it current/ }) as HTMLButtonElement).disabled,
    ).toBe(true);
  });

  it('takes the change back when "No change" is chosen again', async () => {
    const { pending } = field();
    fireEvent.click(screen.getByRole('radio', { name: 'v1.9.0' }));
    await waitFor(() => expect(pending()).not.toBeNull());
    fireEvent.click(screen.getByRole('radio', { name: 'No change' }));
    await waitFor(() => expect(pending()).toBeNull());
  });

  it('keeps versions named like the controls apart from the controls', async () => {
    let pending: PendingSave | null = null;
    const apply = vi.fn(async () => result);
    render(
      <EpicPlanField
        current={null}
        versions={['c:none', '__keep__']}
        apply={apply}
        onPending={(p) => {
          pending = p;
        }}
      />,
    );
    fireEvent.click(screen.getByRole('radio', { name: 'c:none' }));
    await waitFor(() => expect(pending).not.toBeNull());
    await (pending as PendingSave | null)?.();
    expect(apply).toHaveBeenCalledWith({ choice: 'named', version: 'c:none' });
  });

  it('propagates a failing apply so the editor keeps the dialog open', async () => {
    let pending: PendingSave | null = null;
    render(
      <EpicPlanField
        current={null}
        versions={['v1']}
        apply={vi.fn(async () => {
          throw new Error('no');
        })}
        onPending={(p) => {
          pending = p;
        }}
      />,
    );
    fireEvent.click(screen.getByRole('radio', { name: 'v1' }));
    await waitFor(() => expect(pending).not.toBeNull());
    await expect((pending as PendingSave | null)?.()).rejects.toThrow('no');
  });
});
