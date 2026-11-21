import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useSessionLog } from './useSessionLog';

const log = (text: string) => ({ tool: 'codex', exists: true, text, truncated: false, size: 1 });
const reply = (body: unknown, ok = true) => ({
  ok,
  status: ok ? 200 : 500,
  json: async () => body,
});

// The query client hands its results to React on a timer, so a read ends only when timers run.
const settle = () => act(async () => void (await vi.advanceTimersByTimeAsync(10)));

beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe('useSessionLog', () => {
  it('reads the log every interval and stops when the dialog unmounts', async () => {
    const fetchMock = vi.fn(async (_url: string) => reply(log('one')));
    vi.stubGlobal('fetch', fetchMock);
    const { result, unmount } = renderHook(() => useSessionLog('p1', 'codex', 2000));
    await settle();
    expect(result.current.data?.text).toBe('one');
    expect(String(fetchMock.mock.calls[0]?.[0])).toContain('/sessions/log?tool=codex');
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2000);
    });
    expect(fetchMock).toHaveBeenCalledTimes(2);
    unmount();
    await vi.advanceTimersByTimeAsync(10000);
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it('does not start a read while the previous one is in flight', async () => {
    let release: (value: unknown) => void = () => undefined;
    const fetchMock = vi.fn(() => new Promise((resolve) => (release = resolve)));
    vi.stubGlobal('fetch', fetchMock);
    renderHook(() => useSessionLog('p1', 'codex', 2000));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10000);
    });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    release(reply(log('late')));
  });

  it('keeps the last tail and sets failed when a read fails', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(reply(log('kept')))
      .mockRejectedValue(new Error('down'));
    vi.stubGlobal('fetch', fetchMock);
    const { result } = renderHook(() => useSessionLog('p1', 'codex', 2000));
    await settle();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2000);
    });
    await settle();
    expect(result.current.failed).toBe(true);
    expect(result.current.data?.text).toBe('kept');
  });

  it('does not read again while the tab is hidden, and reads again when it is shown', async () => {
    const fetchMock = vi.fn(async () => reply(log('x')));
    vi.stubGlobal('fetch', fetchMock);
    const hidden = vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden');
    renderHook(() => useSessionLog('p1', 'codex', 2000));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(6000);
    });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    hidden.mockReturnValue('visible');
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2000);
    });
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it('starts from nothing for another tool', async () => {
    const fetchMock = vi.fn(async (url: string) =>
      reply(String(url).includes('tool=claude-code') ? log('claude') : log('codex')),
    );
    vi.stubGlobal('fetch', fetchMock);
    const { result, rerender } = renderHook(
      ({ tool }: { tool: 'codex' | 'claude-code' }) => useSessionLog('p1', tool, 2000),
      { initialProps: { tool: 'codex' } },
    );
    await settle();
    expect(result.current.data?.text).toBe('codex');
    rerender({ tool: 'claude-code' });
    expect(result.current.data).toBeNull();
    await settle();
    expect(result.current.data?.text).toBe('claude');
  });
});
