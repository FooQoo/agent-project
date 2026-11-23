import { act, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { SPARRING } from './model';
import {
  applyModeEvent,
  endBrainstormOn,
  refreshModes,
  resetModes,
  setProjectMode,
  startBrainstormOn,
  useProjectMode,
} from './store';

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
    expect(result.current).toEqual(SPARRING);
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
    setProjectMode('p1', { ...SPARRING, mode: 'sleep' });
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

  it('keeps a newer read when an older read completes first', async () => {
    const pending: Array<(response: Response) => void> = [];
    vi.stubGlobal(
      'fetch',
      vi.fn(() => new Promise<Response>((resolve) => pending.push(resolve))),
    );
    const { result } = renderHook(() => useProjectMode('p5'));
    act(() => {
      refreshModes();
    });
    await act(async () => {
      pending[0]?.(Response.json({ mode: 'sparring' }));
    });
    expect(result.current).toBeNull();
    await act(async () => {
      pending[1]?.(Response.json({ mode: 'sleep' }));
    });
    expect(result.current?.mode).toBe('sleep');
  });
});

describe('brainstorm actions', () => {
  const target = { type: 'task', id: '680' } as const;

  it('starts a brainstorm with the target and shows what the server saved', async () => {
    const fetcher = vi.fn(async (_url: string, init?: RequestInit) => {
      expect(init?.method).toBe('PUT');
      expect(JSON.parse(String(init?.body))).toEqual({ mode: 'brainstorm', target });
      return Response.json({ mode: 'brainstorm', manual: 'brainstorm', target });
    });
    vi.stubGlobal('fetch', fetcher);
    setProjectMode('p6', SPARRING);
    const { result } = renderHook(() => useProjectMode('p6'));
    await act(async () => {
      await startBrainstormOn('p6', target);
    });
    expect(result.current?.mode).toBe('brainstorm');
    expect(result.current?.target).toEqual(target);
  });

  it('ends it by saving sparring', async () => {
    const fetcher = vi.fn(async (_url: string, init?: RequestInit) => {
      expect(JSON.parse(String(init?.body))).toEqual({ mode: 'sparring' });
      return Response.json({ mode: 'sparring', manual: 'sparring' });
    });
    vi.stubGlobal('fetch', fetcher);
    setProjectMode('p7', { ...SPARRING, mode: 'brainstorm', target });
    const { result } = renderHook(() => useProjectMode('p7'));
    await act(async () => {
      await endBrainstormOn('p7');
    });
    expect(result.current?.mode).toBe('sparring');
    expect(result.current?.target).toBeNull();
  });

  it('throws on a refusal and leaves the mode as it was', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('{}', { status: 409 })),
    );
    setProjectMode('p8', SPARRING);
    const { result } = renderHook(() => useProjectMode('p8'));
    await expect(startBrainstormOn('p8', target)).rejects.toMatchObject({ status: 409 });
    expect(result.current).toEqual(SPARRING);
  });

  it('follows a brainstorm that the CLI starts', () => {
    setProjectMode('p9', SPARRING);
    const { result } = renderHook(() => useProjectMode('p9'));
    act(() => {
      applyModeEvent({
        data: JSON.stringify({ projects: { p9: { mode: 'brainstorm', target } } }),
      });
    });
    expect(result.current?.target).toEqual(target);
  });
});
