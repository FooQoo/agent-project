import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useLaneWrites } from '../../src/pages/monitor/hooks/useLaneWrites';
import { useLaneOverlay } from '../../src/pages/monitor/hooks/useLaneOverlay';
import { WriteError } from '../../src/shared/api/writeErrors';

const EMPTY_LANES = { backlog: [], todo: [], doing: [], review: [], done: [] } as never;
const writes = {
  move: vi.fn(),
  archive: vi.fn(),
  archiveDone: vi.fn(),
  restore: vi.fn(),
};
const item = (id: number, status = 'todo') =>
  ({ projectId: 'p', task: { id, taskStatus: status } }) as never;

function setup() {
  const onWritten = vi.fn();
  const closePending = vi.fn();
  const hook = renderHook(() => {
    const overlay = useLaneOverlay(EMPTY_LANES);
    const lane = useLaneWrites({
      writes: writes as never,
      projectId: 'p',
      overlay,
      doneItems: [item(1, 'done'), item(2, 'done')],
      closePending,
      onWritten,
    });
    return { lane, overlay };
  });
  return { ...hook, onWritten, closePending };
}

beforeEach(() => vi.resetAllMocks());

describe('phone lane writes', () => {
  it('moves like the PC: reads again after the save, and blocks a second move of a card on its way', async () => {
    let release: () => void = () => {};
    writes.move.mockReturnValue(new Promise<void>((resolve) => (release = resolve)));
    const { result, onWritten } = setup();
    let first: Promise<unknown> = Promise.resolve();
    act(() => {
      first = result.current.lane.moveNow(item(5), 'doing');
      void result.current.lane.moveNow(item(5), 'doing');
    });
    await waitFor(() => expect(writes.move).toHaveBeenCalledTimes(1));
    expect(writes.move).toHaveBeenCalledWith('p', 5, 'doing', 'todo');
    release();
    await act(() => first);
    expect(onWritten).toHaveBeenCalledTimes(1);
    expect(result.current.lane.notice).toBeNull();
  });

  it('shows the notice with the server reason when a move is refused, and reads again', async () => {
    writes.move.mockRejectedValue(new WriteError('Task changed.', 409, 'status_mismatch'));
    const { result, onWritten } = setup();
    await act(() => result.current.lane.moveNow(item(5), 'doing'));
    expect(result.current.lane.notice).toEqual(expect.any(String));
    expect(result.current.lane.notice).not.toBeNull();
    expect(onWritten).toHaveBeenCalledTimes(1);
  });

  it('archives one card, closes the confirmation, and reads again', async () => {
    writes.archive.mockResolvedValue(undefined);
    const { result, onWritten, closePending } = setup();
    let ok = false;
    await act(async () => {
      ok = await result.current.lane.archiveOne(item(3));
    });
    expect(ok).toBe(true);
    expect(writes.archive).toHaveBeenCalledWith('p', 3);
    expect(closePending).toHaveBeenCalled();
    expect(onWritten).toHaveBeenCalledTimes(1);
  });

  it('returns false and reads nothing when an archive is refused', async () => {
    writes.archive.mockRejectedValue(new Error('no'));
    const { result, onWritten, closePending } = setup();
    let ok = true;
    await act(async () => {
      ok = await result.current.lane.archiveOne(item(3));
    });
    expect(ok).toBe(false);
    expect(closePending).not.toHaveBeenCalled();
    expect(onWritten).not.toHaveBeenCalled();
  });

  it('archives the Done lane, and reads again even when it is refused', async () => {
    writes.archiveDone.mockResolvedValueOnce(undefined).mockRejectedValueOnce(new Error('no'));
    const { result, onWritten, closePending } = setup();
    await act(() => result.current.lane.archiveAll());
    expect(writes.archiveDone).toHaveBeenCalledWith('p', [1, 2]);
    expect(closePending).toHaveBeenCalledTimes(1);
    await act(async () => {
      await expect(result.current.lane.archiveAll()).rejects.toThrow('no');
    });
    await waitFor(() => expect(onWritten).toHaveBeenCalledTimes(2));
    expect(closePending).toHaveBeenCalledTimes(1);
  });
});
