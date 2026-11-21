import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ResidentRequestError } from './api';

const api = vi.hoisted(() => ({
  getSessions: vi.fn(),
  getModels: vi.fn(),
  launchSession: vi.fn(),
  stopSession: vi.fn(),
  saveAutoStart: vi.fn(),
}));
vi.mock('./api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./api')>()),
  ...api,
}));

import { useResidentController } from './useResidentController';

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

const health = {
  tool: null,
  state: 'stopped',
  projectId: null,
  model: null,
  reason: null,
  waiting: false,
  backgroundTasks: [],
  pushStalled: [],
  restartCount: 0,
  lastExitAt: null,
  nextRestartAt: null,
  autoStart: null,
};
const snapshot = {
  sessions: [],
  counts: { total: 0, connected: 0, byOrigin: { board: 0, external: 0 } },
  launched: [],
  health,
};

async function ready() {
  api.getSessions.mockResolvedValue(snapshot);
  api.getModels.mockResolvedValue({ models: [], reason: null });
  const hook = renderHook(() => useResidentController('p1'));
  await waitFor(() => expect(hook.result.current.choice).not.toBeNull());
  return hook;
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

describe('useResidentController', () => {
  it('starts once, stays pending until the sessions are read again, and refetches them', async () => {
    const gate = deferred<unknown>();
    api.launchSession.mockReturnValue(gate.promise);
    const { result } = await ready();
    const reads = api.getSessions.mock.calls.length;
    act(() => {
      result.current.start();
      result.current.start();
    });
    await waitFor(() => expect(result.current.pending).toBe('starting'));
    gate.resolve({});
    await waitFor(() => expect(result.current.pending).toBeNull());
    expect(api.launchSession).toHaveBeenCalledTimes(1);
    expect(api.getSessions.mock.calls.length).toBeGreaterThan(reads);
  });

  it('shows the server reason of a refused launch and still reads the sessions again', async () => {
    api.launchSession.mockRejectedValue(new ResidentRequestError(409, 'busy', 'Already running'));
    const { result } = await ready();
    const reads = api.getSessions.mock.calls.length;
    act(() => result.current.start());
    await waitFor(() => expect(result.current.launchProblem?.reason).toBe('Already running'));
    await waitFor(() => expect(result.current.pending).toBeNull());
    expect(api.getSessions.mock.calls.length).toBeGreaterThan(reads);
  });

  it('shows a refused stop as a problem', async () => {
    api.stopSession.mockRejectedValue(new ResidentRequestError(500, 'internal', 'No'));
    const { result } = await ready();
    act(() => result.current.stop());
    await waitFor(() => expect(result.current.problem?.reason).toBe('No'));
    expect(result.current.pending).toBeNull();
  });

  it('saves the auto-start choice and reads the sessions again, or shows the problem', async () => {
    api.saveAutoStart
      .mockRejectedValueOnce(new ResidentRequestError(500, 'save_failed', 'Disk full'))
      .mockResolvedValueOnce({ autoStart: null });
    const { result } = await ready();
    act(() => result.current.setAutoStart(true));
    await waitFor(() => expect(result.current.problem?.reason).toBe('Disk full'));
    const reads = api.getSessions.mock.calls.length;
    act(() => result.current.setAutoStart(true));
    await waitFor(() => expect(api.getSessions.mock.calls.length).toBeGreaterThan(reads));
    await waitFor(() => expect(result.current.savingAuto).toBe(false));
    expect(result.current.problem).toBeNull();
  });
});
