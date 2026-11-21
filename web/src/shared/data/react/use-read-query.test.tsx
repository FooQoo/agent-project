import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ChangedEvent } from '../api/contracts';
import { fakeCore } from './test-support';
import { useReadQuery } from './use-read-query';

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const P1 = '0f1e2d3c4b5a69788796a5b4c3d2e1f0';
const P2 = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

const changed = (projectId: string, resource: 'task' | 'messages' = 'task'): ChangedEvent => ({
  instanceId: 'i',
  seq: 1,
  projectId,
  changes: [{ resource, id: '1', facets: ['summary'] }],
});

describe('useReadQuery', () => {
  it('reads once, and again only for a notification that concerns it or for a reset', async () => {
    const { wrapper, emit } = fakeCore();
    const mine = vi.fn(async () => 'mine');
    const other = vi.fn(async () => 'other');
    const a = renderHook(
      () =>
        useReadQuery({
          queryKey: ['a', P1],
          queryFn: mine,
          changedBy: (event) => event.projectId === P1,
        }),
      { wrapper },
    );
    renderHook(
      () =>
        useReadQuery({
          queryKey: ['b', P2],
          queryFn: other,
          changedBy: (event) => event.projectId === P2,
        }),
      { wrapper },
    );
    await waitFor(() => expect(a.result.current.data).toBe('mine'));
    await emit(changed(P1));
    await waitFor(() => expect(mine).toHaveBeenCalledTimes(2));
    // The other query was not read again by a change of the first project.
    expect(other).toHaveBeenCalledTimes(1);
    await emit(null);
    await waitFor(() => expect(mine).toHaveBeenCalledTimes(3));
    await waitFor(() => expect(other).toHaveBeenCalledTimes(2));
  });

  it('reads again on a reset, but not on an ordinary change, when it names no change filter', async () => {
    const { wrapper, emit } = fakeCore();
    const read = vi.fn(async () => 'list');
    renderHook(() => useReadQuery({ queryKey: ['list'], queryFn: read }), { wrapper });
    await waitFor(() => expect(read).toHaveBeenCalledTimes(1));
    await emit(changed(P1));
    expect(read).toHaveBeenCalledTimes(1);
    await emit(null);
    await waitFor(() => expect(read).toHaveBeenCalledTimes(2));
  });

  it('joins a read that is running instead of starting another', async () => {
    const { wrapper, emit } = fakeCore();
    let release: (value: string) => void = () => {};
    const read = vi.fn(() => new Promise<string>((resolve) => (release = resolve)));
    const { result } = renderHook(
      () => useReadQuery({ queryKey: ['a'], queryFn: read, changedBy: () => true }),
      { wrapper },
    );
    await waitFor(() => expect(read).toHaveBeenCalledTimes(1));
    await emit(changed(P1));
    await emit(changed(P1));
    expect(read).toHaveBeenCalledTimes(1);
    await act(async () => release('done'));
    await waitFor(() => expect(result.current.data).toBe('done'));
  });

  it('shows a failure at once, with no hidden retries, and reads again on request', async () => {
    const { wrapper } = fakeCore();
    const read = vi.fn().mockRejectedValueOnce(new Error('down')).mockResolvedValue('up');
    const { result } = renderHook(() => useReadQuery({ queryKey: ['a'], queryFn: read }), {
      wrapper,
    });
    await waitFor(() => expect(result.current.isError).toBe(true));
    expect(read).toHaveBeenCalledTimes(1);
    await act(async () => void (await result.current.refetch({ cancelRefetch: false })));
    await waitFor(() => expect(result.current.data).toBe('up'));
  });

  it('does not show the data of another project after a switch', async () => {
    const { wrapper } = fakeCore();
    const read = vi.fn(async (id: string) => `data of ${id}`);
    const { result, rerender } = renderHook(
      ({ id }: { id: string }) => useReadQuery({ queryKey: ['a', id], queryFn: () => read(id) }),
      { wrapper, initialProps: { id: P1 } },
    );
    await waitFor(() => expect(result.current.data).toBe(`data of ${P1}`));
    rerender({ id: P2 });
    expect(result.current.data).toBeUndefined();
    await waitFor(() => expect(result.current.data).toBe(`data of ${P2}`));
  });

  it('works without a data provider, with a client of its own', async () => {
    const read = vi.fn(async () => 'alone');
    const { result } = renderHook(() => useReadQuery({ queryKey: ['a'], queryFn: read }));
    await waitFor(() => expect(result.current.data).toBe('alone'));
  });
});
