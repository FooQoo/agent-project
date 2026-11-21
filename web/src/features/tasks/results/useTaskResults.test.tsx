import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { fakeCore } from '@/shared/data/react/test-support';
import { useTaskResults } from './useTaskResults';

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const P1 = '0f1e2d3c4b5a69788796a5b4c3d2e1f0';

describe('useTaskResults', () => {
  it("reads a task's results again when the history grows, and only while the tab is shown", async () => {
    const { wrapper } = fakeCore();
    const fetchMock = vi.fn(async () => ({
      ok: true,
      status: 200,
      json: async () => ({ results: [] }),
    }));
    vi.stubGlobal('fetch', fetchMock);
    const { result, rerender } = renderHook(
      ({ key, enabled }: { key: number; enabled: boolean }) => useTaskResults(P1, 7, key, enabled),
      { wrapper, initialProps: { key: 1, enabled: false } },
    );
    expect(fetchMock).not.toHaveBeenCalled();
    rerender({ key: 1, enabled: true });
    await waitFor(() => expect(result.current.status).toBe('ready'));
    expect(fetchMock).toHaveBeenCalledTimes(1);
    rerender({ key: 2, enabled: true });
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
  });

  it('keeps the results and says the refresh failed; Retry reads without hidden retries', async () => {
    const { wrapper } = fakeCore();
    const body = { ok: true, status: 200, json: async () => ({ results: [] }) };
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(body)
      .mockResolvedValueOnce({ ok: false, status: 500, json: async () => ({}) })
      .mockResolvedValue(body);
    vi.stubGlobal('fetch', fetchMock);
    const { result, rerender } = renderHook(
      ({ key }: { key: number }) => useTaskResults(P1, 7, key),
      { wrapper, initialProps: { key: 1 } },
    );
    await waitFor(() => expect(result.current.status).toBe('ready'));
    rerender({ key: 2 });
    await waitFor(() => expect(result.current.refreshFailed).toBe(true));
    expect(result.current.status).toBe('ready');
    expect(fetchMock).toHaveBeenCalledTimes(2);
    act(() => result.current.retry());
    await waitFor(() => expect(result.current.refreshFailed).toBe(false));
  });
});
