import { act, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { applyModeEvent, refreshModes, resetModes, setProjectMode, useProjectMode } from './store';

afterEach(() => {
  resetModes();
  vi.unstubAllGlobals();
});

describe('useProjectMode', () => {
  it('reads the mode once and follows the mode events', async () => {
    const fetcher = vi.fn(async () => Response.json({ mode: 'sparring' }));
    vi.stubGlobal('fetch', fetcher);
    const { result } = renderHook(() => useProjectMode('p1'));
    expect(result.current).toBeNull();
    await act(async () => {});
    expect(result.current).toEqual({ mode: 'sparring', since: null });
    act(() => {
      applyModeEvent({
        data: JSON.stringify({
          projects: { p1: { mode: 'sleep', since: '2026-11-24T01:00:00Z' } },
        }),
      });
    });
    expect(result.current?.mode).toBe('sleep');
    expect(fetcher).toHaveBeenCalledTimes(1);
  });

  it('keeps a mode it already knows without reading again', () => {
    const fetcher = vi.fn();
    vi.stubGlobal('fetch', fetcher);
    setProjectMode('p1', { mode: 'sleep', since: null });
    const { result } = renderHook(() => useProjectMode('p1'));
    expect(result.current?.mode).toBe('sleep');
    expect(fetcher).not.toHaveBeenCalled();
  });

  it('stays unknown when the read fails', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('no', { status: 500 })),
    );
    const { result } = renderHook(() => useProjectMode('p2'));
    await act(async () => {});
    expect(result.current).toBeNull();
  });

  it('retries a project whose first read failed when the stream reconnects', async () => {
    const fetcher = vi
      .fn()
      .mockResolvedValueOnce(new Response('no', { status: 500 }))
      .mockResolvedValueOnce(Response.json({ mode: 'sleep' }));
    vi.stubGlobal('fetch', fetcher);
    const { result } = renderHook(() => useProjectMode('p3'));
    await act(async () => {});
    expect(result.current).toBeNull();
    await act(async () => {
      refreshModes();
    });
    expect(result.current?.mode).toBe('sleep');
  });

  it('does not let an older read overwrite a newer update', async () => {
    let finish: (response: Response) => void = () => undefined;
    vi.stubGlobal(
      'fetch',
      vi.fn(() => new Promise<Response>((resolve) => (finish = resolve))),
    );
    const { result } = renderHook(() => useProjectMode('p4'));
    act(() => {
      applyModeEvent({
        data: JSON.stringify({ projects: { p4: { mode: 'sleep', since: null } } }),
      });
    });
    await act(async () => {
      finish(Response.json({ mode: 'sparring' }));
    });
    expect(result.current?.mode).toBe('sleep');
  });
});
