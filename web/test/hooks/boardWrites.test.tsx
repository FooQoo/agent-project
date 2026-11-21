import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const api = vi.hoisted(() => ({
  reorderTask: vi.fn(),
  editTask: vi.fn(),
  editTaskMultipart: vi.fn(),
  setDraft: vi.fn(),
}));
vi.mock('../../src/app/api', () => ({
  reorderTask: api.reorderTask,
  editTask: api.editTask,
  editTaskMultipart: api.editTaskMultipart,
}));
vi.mock('../../src/features/tasks/api/edit', () => ({ setDraft: api.setDraft }));
vi.mock('../../src/features/messages/ask/sendRequest', () => ({ sendMessageRequest: vi.fn() }));

import { useDialogSubmit, useReorder } from '../../src/app/useBoardWrites';
import { useFinalizeDraft } from '../../src/features/tasks/detail/useFinalizeDraft';
import { WriteError } from '../../src/shared/api/writeErrors';

beforeEach(() => vi.resetAllMocks());

describe('useReorder', () => {
  it('reads the project again after a save, and also after a refusal that it does not show', async () => {
    const refresh = vi.fn();
    api.reorderTask.mockResolvedValueOnce({}).mockRejectedValueOnce(new Error('no'));
    const { result } = renderHook(() => useReorder(refresh));
    act(() => result.current.mutate({ projectId: 'p', id: 1, beforeId: null }));
    await waitFor(() => expect(refresh).toHaveBeenCalledTimes(1));
    act(() => result.current.mutate({ projectId: 'p', id: 1, beforeId: 2 }));
    await waitFor(() => expect(refresh).toHaveBeenCalledTimes(2));
    expect(refresh).toHaveBeenCalledWith('p');
  });
});

describe('useDialogSubmit', () => {
  const session = {
    dialog: { kind: 'edit', task: { id: 7 } },
    operation: { projectId: 'p' },
  } as never;

  it('saves an edit and reads the project again', async () => {
    const refresh = vi.fn();
    api.editTask.mockResolvedValue({});
    const { result } = renderHook(() => useDialogSubmit(session, refresh, vi.fn()));
    await act(() => result.current({ title: 'T' } as never));
    expect(api.editTask).toHaveBeenCalledWith('p', 7, expect.objectContaining({ title: 'T' }));
    expect(refresh).toHaveBeenCalledWith('p');
  });

  it('rejects with the server reason and reads nothing when the save is refused', async () => {
    const refresh = vi.fn();
    api.editTask.mockRejectedValue(new WriteError('Version changed.', 409, 'version_mismatch'));
    const { result } = renderHook(() => useDialogSubmit(session, refresh, vi.fn()));
    await expect(result.current({ title: 'T' } as never)).rejects.toMatchObject({
      message: 'Version changed.',
      status: 409,
      code: 'version_mismatch',
    });
    expect(refresh).not.toHaveBeenCalled();
  });
});

describe('useFinalizeDraft', () => {
  it('sends one request at a time, keeps a failure to show, and clears it on the next try', async () => {
    let answer: (value: unknown) => void = () => {};
    api.setDraft.mockReturnValueOnce(new Promise((resolve) => (answer = resolve)));
    const changed = vi.fn();
    const { result } = renderHook(() => useFinalizeDraft('p', 7, changed));
    act(() => {
      void result.current.finalizeDraft();
      void result.current.finalizeDraft();
    });
    await waitFor(() => expect(result.current.draftSaving).toBe(true));
    expect(api.setDraft).toHaveBeenCalledTimes(1);
    await act(async () => answer({}));
    await waitFor(() => expect(changed).toHaveBeenCalledTimes(1));
    expect(result.current.draftFailed).toBe(false);

    api.setDraft.mockRejectedValueOnce(new Error('no'));
    await act(() => result.current.finalizeDraft());
    await waitFor(() => expect(result.current.draftFailed).toBe(true));
    expect(changed).toHaveBeenCalledTimes(1);
    api.setDraft.mockResolvedValueOnce({});
    await act(() => result.current.finalizeDraft());
    await waitFor(() => expect(result.current.draftFailed).toBe(false));
  });
});
