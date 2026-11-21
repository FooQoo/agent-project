import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { fakeCore } from '@/shared/data/react/test-support';
import { useMessageDetail } from './useMessageDetail';

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const P1 = '0f1e2d3c4b5a69788796a5b4c3d2e1f0';

describe('useMessageDetail', () => {
  it('reads the opened message, and shows a failure until reload succeeds', async () => {
    const { wrapper } = fakeCore();
    const detail = { id: 'm1' } as never;
    const read = vi.fn().mockRejectedValueOnce(new Error('no')).mockResolvedValue(detail);
    const { result, rerender } = renderHook(
      ({ id }: { id: string | null }) => useMessageDetail(read, P1, id),
      { wrapper, initialProps: { id: null as string | null } },
    );
    expect(read).not.toHaveBeenCalled();
    rerender({ id: 'm1' });
    await waitFor(() => expect(result.current.state).toBe('failed'));
    act(() => result.current.reload());
    await waitFor(() => expect(result.current.state).toBe('ready'));
    rerender({ id: null });
    expect(result.current.detail).toBeNull();
  });
});
