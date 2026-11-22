import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import '../../../shared/i18n';
import type { PendingSave } from '../../../shared/ui/editor/planField';
import type { EpicVersionResult } from '../api/versions';
import { EpicPlanField } from './EpicPlanField';

afterEach(cleanup);

const members = [
  { id: 1, title: 'Plan one', taskStatus: 'todo', plannedVersion: null },
  { id: 2, title: 'Plan two', taskStatus: 'doing', plannedVersion: null },
  { id: 3, title: 'Plan three', taskStatus: 'done', plannedVersion: null },
  { id: 4, title: 'Plan four', taskStatus: 'todo', plannedVersion: 'v1.9.0' },
];

const dry: EpicVersionResult = {
  changed: [1, 2],
  unchanged: [],
  skipped: [3, 4],
  skippedReasons: { '3': 'done', '4': 'planned' },
  dryRun: true,
};

function field(preview = vi.fn(async () => dry)) {
  let pending: PendingSave | null = null;
  const onPending = vi.fn((next: PendingSave | null) => {
    pending = next;
  });
  const apply = vi.fn(async () => ({ ...dry, dryRun: false }));
  render(
    <EpicPlanField
      members={members}
      current="v1.8.0"
      versions={['v1.8.0', 'v1.9.0']}
      preview={preview}
      apply={apply}
      onPending={onPending}
    />,
  );
  return { preview, apply, onPending, pending: () => pending as PendingSave | null };
}

describe('the planned versions of an epic, in its editor', () => {
  it('changes nothing and asks nothing until a version is chosen', () => {
    const { preview, pending } = field();
    expect(preview).not.toHaveBeenCalled();
    expect(pending()).toBeNull();
  });

  it('shows the dry run of each task before anything is saved', async () => {
    const { preview, apply } = field();
    fireEvent.click(screen.getByRole('radio', { name: /Make it current/ }));
    expect(await screen.findByText('2 of 4 tasks will change to v1.8.0')).toBeTruthy();
    expect(preview).toHaveBeenCalledWith({ choice: 'current' }, 'unplanned-only', [1, 2, 3, 4]);
    expect(screen.getByText('Skipped: done')).toBeTruthy();
    expect(screen.getByText('Skipped: already planned')).toBeTruthy();
    expect(apply).not.toHaveBeenCalled();
  });

  it('applies the checked rows with the same body and no dry run when the editor saves', async () => {
    const { apply, pending } = field();
    fireEvent.click(screen.getByRole('radio', { name: 'v1.9.0' }));
    await screen.findByText('2 of 4 tasks will change to v1.9.0');
    fireEvent.click(screen.getByRole('checkbox', { name: /Plan two/ }));
    expect(screen.getByText('1 of 4 tasks will change to v1.9.0')).toBeTruthy();
    await waitFor(() => expect(pending()).not.toBeNull());
    await pending()?.();
    expect(apply).toHaveBeenCalledWith(
      { choice: 'named', version: 'v1.9.0' },
      'unplanned-only',
      [1],
    );
  });

  it('asks again when the scope changes', async () => {
    const { preview } = field();
    fireEvent.click(screen.getByRole('radio', { name: 'v1.9.0' }));
    await screen.findByText('2 of 4 tasks will change to v1.9.0');
    fireEvent.click(screen.getByRole('radio', { name: /Every task in the epic/ }));
    await waitFor(() =>
      expect(preview).toHaveBeenLastCalledWith(
        { choice: 'named', version: 'v1.9.0' },
        'all',
        [1, 2, 3, 4],
      ),
    );
  });

  it('has nothing to save when no task would change', async () => {
    const { pending } = field(
      vi.fn(
        async (): Promise<EpicVersionResult> => ({
          changed: [],
          unchanged: [1, 2],
          skipped: [3, 4],
          skippedReasons: { '3': 'done', '4': 'planned' },
          dryRun: true,
        }),
      ),
    );
    fireEvent.click(screen.getByRole('radio', { name: /Make it current/ }));
    expect(await screen.findByText('No task will change')).toBeTruthy();
    expect(pending()).toBeNull();
    expect(screen.getAllByText('No change').length).toBeGreaterThan(0);
  });

  it('takes the change back when "No change" is chosen again', async () => {
    const { pending } = field();
    fireEvent.click(screen.getByRole('radio', { name: 'v1.9.0' }));
    await waitFor(() => expect(pending()).not.toBeNull());
    fireEvent.click(screen.getByRole('radio', { name: 'No change' }));
    await waitFor(() => expect(pending()).toBeNull());
  });

  it('says so when the check fails', async () => {
    field(
      vi.fn(async () => {
        throw new Error('nope');
      }),
    );
    fireEvent.click(screen.getByRole('radio', { name: 'v1.9.0' }));
    expect((await screen.findByRole('alert')).textContent).toContain('Nothing was changed');
  });

  it('keeps versions named like the controls apart from the controls', async () => {
    const { pending, preview } = field();
    cleanup();
    let next: PendingSave | null = null;
    render(
      <EpicPlanField
        members={members}
        current={null}
        versions={['__none__', 'c:none', '__keep__']}
        preview={preview}
        apply={vi.fn(async () => ({ ...dry, dryRun: false }))}
        onPending={(p) => {
          next = p;
        }}
      />,
    );
    fireEvent.click(screen.getByRole('radio', { name: 'c:none' }));
    await waitFor(() =>
      expect(preview).toHaveBeenLastCalledWith(
        { choice: 'named', version: 'c:none' },
        'unplanned-only',
        [1, 2, 3, 4],
      ),
    );
    fireEvent.click(screen.getByRole('radio', { name: '__none__' }));
    await waitFor(() =>
      expect(preview).toHaveBeenLastCalledWith(
        { choice: 'named', version: '__none__' },
        'unplanned-only',
        [1, 2, 3, 4],
      ),
    );
    expect(pending()).toBeNull();
    expect(next).not.toBeNull();
  });

  it('keeps a task unchecked when another client changes the members', async () => {
    const preview = vi.fn(async () => dry);
    const props = {
      current: 'v1.8.0',
      versions: ['v1.8.0', 'v1.9.0'],
      preview,
      apply: vi.fn(async () => ({ ...dry, dryRun: false })),
      onPending: vi.fn(),
    };
    const view = render(<EpicPlanField members={members} {...props} />);
    fireEvent.click(screen.getByRole('radio', { name: 'v1.9.0' }));
    await screen.findByText('2 of 4 tasks will change to v1.9.0');
    fireEvent.click(screen.getByRole('checkbox', { name: /Plan two/ }));
    expect(screen.getByText('1 of 4 tasks will change to v1.9.0')).toBeTruthy();
    // Task 4 is planned by someone else meanwhile: the members change, the checks stay.
    const next = members.map((m) => (m.id === 3 ? { ...m, plannedVersion: 'v1.9.0' } : m));
    view.rerender(<EpicPlanField members={next} {...props} />);
    await waitFor(() => expect(preview).toHaveBeenCalledTimes(2));
    await screen.findByText('1 of 4 tasks will change to v1.9.0');
    expect(
      (screen.getByRole('checkbox', { name: /Plan two/ }) as HTMLInputElement).getAttribute(
        'aria-checked',
      ),
    ).toBe('false');
    // Choosing another scope starts a different operation: every task is checked again.
    fireEvent.click(screen.getByRole('radio', { name: /Every task in the epic/ }));
    await screen.findByText('2 of 4 tasks will change to v1.9.0');
  });
});
