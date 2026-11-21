import { act, renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { useWrite } from '../../src/shared/data/react/use-write';

// The Docs dialog shows its "could not open" notices from the state of these two writes.
describe('opening Docs as a write', () => {
  it('keeps the failed path so the notice shows only for that file, and clears it on the next try', async () => {
    const open = vi.fn().mockRejectedValueOnce(new Error('no')).mockResolvedValueOnce(undefined);
    const { result } = renderHook(() =>
      useWrite<void, string>({ mutationKey: ['docs-open-file'], mutationFn: (path) => open(path) }),
    );
    act(() => result.current.mutate('a.md'));
    await waitFor(() => expect(result.current.isError).toBe(true));
    expect(result.current.variables).toBe('a.md');
    act(() => result.current.mutate('b.md'));
    await waitFor(() => expect(result.current.isError).toBe(false));
    expect(open).toHaveBeenCalledTimes(2);
  });
});
