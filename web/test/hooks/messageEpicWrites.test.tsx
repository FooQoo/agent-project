import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const api = vi.hoisted(() => ({ updateEpic: vi.fn(), deleteEpic: vi.fn() }));
vi.mock('../../src/features/epics/api', async (importOriginal) => ({
  ...(await importOriginal<object>()),
  updateEpic: api.updateEpic,
  deleteEpic: api.deleteEpic,
}));

import { useEpicWrites } from '../../src/features/epics/editor/epicApi';
import { useMessageWrites } from '../../src/features/tasks/messages/useMessageWrites';
import type { MessagesApi } from '../../src/features/tasks/messages/model/messages';
import { WriteError } from '../../src/shared/api/writeErrors';

beforeEach(() => vi.resetAllMocks());

describe('useEpicWrites', () => {
  it('reads the project again after a write that went through, not after a refusal', async () => {
    const changed = vi.fn();
    api.updateEpic.mockResolvedValueOnce({ epic: {} });
    api.deleteEpic.mockRejectedValueOnce(new WriteError('in use', 409, 'epic_in_use', null));
    const { result } = renderHook(() => useEpicWrites(changed));
    const epic = result.current('p', 'E');
    await act(() => epic.saveWaves([[1]]));
    expect(api.updateEpic).toHaveBeenCalledWith('p', 'E', { waves: [[1]] });
    expect(changed).toHaveBeenCalledWith('p');
    await expect(act(() => epic.remove())).rejects.toMatchObject({
      message: 'in use',
      code: 'epic_in_use',
    });
    expect(changed).toHaveBeenCalledTimes(1);
  });
});

describe('useMessageWrites', () => {
  const makeApi = () =>
    ({
      read: vi.fn(),
      answer: vi.fn(),
      withdraw: vi.fn(),
      summaries: undefined,
    }) as unknown as MessagesApi;

  it('calls the injected api and refuses a second answer while one is on its way', async () => {
    const messages = makeApi();
    let finish!: () => void;
    vi.mocked(messages.answer).mockReturnValue(new Promise((r) => (finish = () => r({} as never))));
    const { result } = renderHook(() => useMessageWrites(messages, 'p'));
    let first!: Promise<boolean>;
    act(() => {
      first = result.current.answer([{ questionId: 'q', text: 'a' }], []);
    });
    await waitFor(() => expect(result.current.busy).toBe(true));
    await expect(result.current.answer([{ questionId: 'q' }], [])).resolves.toBe(false);
    expect(messages.answer).toHaveBeenCalledTimes(1);
    await act(async () => {
      finish();
      await first;
    });
    expect(messages.answer).toHaveBeenCalledWith('p', [{ questionId: 'q', text: 'a' }], []);
    await waitFor(() => expect(result.current.busy).toBe(false));
  });

  it('rejects with the error of the api so the screen can show its reason', async () => {
    const messages = makeApi();
    const error = new WriteError('gone', 404, 'not_found', null);
    vi.mocked(messages.withdraw).mockRejectedValue(error);
    const { result } = renderHook(() => useMessageWrites(messages, 'p'));
    await expect(act(() => result.current.withdraw('m'))).rejects.toBe(error);
    expect(messages.withdraw).toHaveBeenCalledWith('p', 'm');
    await waitFor(() => expect(result.current.busy).toBe(false));
  });
});
