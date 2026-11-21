import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  getPushStatus: vi.fn(),
  enablePush: vi.fn(),
  disablePush: vi.fn(),
  hasBrowserSubscription: vi.fn(),
}));
vi.mock('./api', () => ({ getPushStatus: mocks.getPushStatus }));
vi.mock('./subscribe', () => ({
  enablePush: mocks.enablePush,
  disablePush: mocks.disablePush,
  hasBrowserSubscription: mocks.hasBrowserSubscription,
}));

import { usePushNotifications } from './usePushNotifications';

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

async function ready() {
  mocks.getPushStatus.mockResolvedValue({ available: true, publicKey: 'k', subscribed: false });
  mocks.hasBrowserSubscription.mockResolvedValue(false);
  const hook = renderHook(() => usePushNotifications());
  await waitFor(() => expect(mocks.getPushStatus).toHaveBeenCalled());
  await waitFor(() => expect(hook.result.current.view).toBeDefined());
  return hook;
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

describe('usePushNotifications', () => {
  it('turns on once, stays busy until the status is read again, and reports a failure', async () => {
    const gate = deferred<string>();
    mocks.enablePush.mockReturnValueOnce(gate.promise);
    const { result } = await ready();
    const reads = mocks.getPushStatus.mock.calls.length;
    await waitFor(() => expect(mocks.getPushStatus.mock.results.length).toBeGreaterThan(0));
    act(() => {
      result.current.enable();
      result.current.enable();
    });
    await waitFor(() => expect(result.current.busy).toBe(true));
    gate.resolve('failed');
    await waitFor(() => expect(result.current.busy).toBe(false));
    expect(mocks.enablePush).toHaveBeenCalledTimes(1);
    expect(result.current.failed).toBe(true);
    expect(mocks.getPushStatus.mock.calls.length).toBeGreaterThan(reads);
  });

  it('turns off, clears the failure and reads the status again', async () => {
    mocks.disablePush.mockResolvedValue(false).mockResolvedValueOnce(false);
    const { result } = await ready();
    act(() => result.current.disable());
    await waitFor(() => expect(result.current.failed).toBe(true));
    mocks.disablePush.mockResolvedValue(true);
    await waitFor(() => expect(result.current.busy).toBe(false));
    act(() => result.current.disable());
    await waitFor(() => expect(result.current.failed).toBe(false));
  });
});
