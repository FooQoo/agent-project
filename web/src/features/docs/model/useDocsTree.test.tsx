import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { fakeCore } from '@/shared/data/react/test-support';
import { useDocsTree } from './useDocsTree';

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const P1 = '0f1e2d3c4b5a69788796a5b4c3d2e1f0';

describe('useDocsTree', () => {
  it('reads the docs list when the dialog opens, and again on reload', async () => {
    const { wrapper } = fakeCore();
    const tree = { nodes: [], truncated: false };
    const fetchMock = vi.fn(async () => ({ ok: true, status: 200, json: async () => tree }));
    vi.stubGlobal('fetch', fetchMock);
    const { result } = renderHook(() => useDocsTree(P1), { wrapper });
    expect(result.current.status).toBe('loading');
    await waitFor(() => expect(result.current.status).toBe('ready'));
    act(() => result.current.reload());
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
  });
});
