import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { useEffect } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import '../../../shared/i18n';
import type { Epic } from '../../../shared/api/contracts';
import type { PlanFieldArgs } from '../../../shared/ui/editor/planField';
import { EpicDialog } from './EpicDialog';
import type { EpicApi } from './epicApi';

afterEach(cleanup);

const epic = {
  id: 'E1',
  name: 'Epic one',
  color: '#7C3AED',
  draft: null,
  document: 'Text',
  body: 'Text',
  waves: [],
  wavesError: false,
  attachments: [],
  createdAt: '',
  updatedAt: '',
} as Epic;

function setup(planSave: () => Promise<void>) {
  const api = {
    save: vi.fn(async () => undefined),
    saveWaves: vi.fn(async () => undefined),
    addImages: vi.fn(async () => undefined),
    removeImage: vi.fn(async () => undefined),
    enterDraft: vi.fn(async () => epic),
    leaveDraft: vi.fn(async () => undefined),
    remove: vi.fn(async () => undefined),
  } as unknown as EpicApi & { save: ReturnType<typeof vi.fn> };
  // A field that reports one pending change as soon as it is shown.
  const Field = ({ onPending }: PlanFieldArgs) => {
    useEffect(() => onPending(planSave), [onPending]);
    return <p>plan field</p>;
  };
  render(
    <EpicDialog
      epic={epic}
      tasks={[]}
      api={api}
      projectId="p"
      onOpenTask={() => {}}
      onClose={() => {}}
      renderTask={() => null}
      renderPlan={(args) => <Field {...args} />}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Planned versions' }));
  return api;
}

const save = () => screen.getByRole('button', { name: 'Save' }) as HTMLButtonElement;

describe('the epic editor saving the planned versions', () => {
  it('saves a plan-only change without sending the epic', async () => {
    const plan = vi.fn(async () => undefined);
    const api = setup(plan);
    await waitFor(() => expect(save().disabled).toBe(false));
    fireEvent.click(save());
    await waitFor(() => expect(plan).toHaveBeenCalled());
    expect(api.save).not.toHaveBeenCalled();
  });

  it('does not enable Save through the plan alone while the editor has changes', async () => {
    const api = setup(async () => undefined);
    fireEvent.change(screen.getByRole('textbox', { name: 'Title' }), {
      target: { value: 'Renamed' },
    });
    // The draft is being entered: Save waits for the lock instead of going through the plan branch.
    expect(api.save).not.toHaveBeenCalled();
    await waitFor(() => expect(api.enterDraft).toHaveBeenCalled());
  });

  it('does not send the epic again when only the plan step failed and nothing new was typed', async () => {
    const plan = vi.fn().mockRejectedValueOnce(new Error('no')).mockResolvedValue(undefined);
    const api = setup(plan);
    fireEvent.change(screen.getByRole('textbox', { name: 'Title' }), {
      target: { value: 'Renamed' },
    });
    await waitFor(() => expect(save().disabled).toBe(false));
    fireEvent.click(save());
    await waitFor(() => expect(plan).toHaveBeenCalledTimes(1));
    expect(api.save).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(save().disabled).toBe(false));
    fireEvent.click(save());
    await waitFor(() => expect(plan).toHaveBeenCalledTimes(2));
    expect(api.save).toHaveBeenCalledTimes(1);
  });

  it('sends the epic again when there are new edits before the retry', async () => {
    const plan = vi.fn().mockRejectedValueOnce(new Error('no')).mockResolvedValue(undefined);
    const api = setup(plan);
    const name = () => screen.getByRole('textbox', { name: 'Title' });
    fireEvent.change(name(), { target: { value: 'Renamed' } });
    await waitFor(() => expect(save().disabled).toBe(false));
    fireEvent.click(save());
    await waitFor(() => expect(plan).toHaveBeenCalledTimes(1));
    fireEvent.change(name(), { target: { value: 'Renamed again' } });
    await waitFor(() => expect(save().disabled).toBe(false));
    fireEvent.click(save());
    await waitFor(() => expect(plan).toHaveBeenCalledTimes(2));
    expect(api.save).toHaveBeenCalledTimes(2);
  });

  it('still applies the accepted plan when the board clears it while the epic is being saved', async () => {
    const plan = vi.fn(async () => undefined);
    let report: PlanFieldArgs['onPending'] = () => {};
    const api = {
      save: vi.fn(async () => {
        // A Summary update makes the field drop its pending change mid-save.
        report(null);
      }),
      saveWaves: vi.fn(async () => undefined),
      addImages: vi.fn(async () => undefined),
      removeImage: vi.fn(async () => undefined),
      enterDraft: vi.fn(async () => epic),
      leaveDraft: vi.fn(async () => undefined),
      remove: vi.fn(async () => undefined),
    } as unknown as EpicApi;
    const Field = ({ onPending }: PlanFieldArgs) => {
      report = onPending;
      useEffect(() => onPending(plan), [onPending]);
      return <p>plan field</p>;
    };
    render(
      <EpicDialog
        epic={epic}
        tasks={[]}
        api={api}
        projectId="p"
        onOpenTask={() => {}}
        onClose={() => {}}
        renderTask={() => null}
        renderPlan={(args) => <Field {...args} />}
      />,
    );
    fireEvent.change(screen.getByRole('textbox', { name: 'Title' }), {
      target: { value: 'Renamed' },
    });
    await waitFor(() => expect(save().disabled).toBe(false));
    fireEvent.click(save());
    await waitFor(() => expect(plan).toHaveBeenCalledTimes(1));
  });
});
