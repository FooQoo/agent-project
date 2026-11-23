import { act, renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { useVersionFilter } from './useVersionFilter';

describe('useVersionFilter', () => {
  it('starts on the release version, or All when none is set', () => {
    expect(renderHook(() => useVersionFilter('p', 'v1.8.0')).result.current.filter).toBe('v1.8.0');
    expect(renderHook(() => useVersionFilter('p', null)).result.current.filter).toBeNull();
  });

  it('keeps a choice, including All, and drops it for another project', () => {
    const { result, rerender } = renderHook(({ id }) => useVersionFilter(id, 'v1.8.0'), {
      initialProps: { id: 'a' },
    });
    act(() => result.current.setFilter(null));
    expect(result.current.filter).toBeNull();
    act(() => result.current.setFilter('v1.9.0'));
    expect(result.current.filter).toBe('v1.9.0');
    rerender({ id: 'b' });
    expect(result.current.filter).toBe('v1.8.0');
  });
});
