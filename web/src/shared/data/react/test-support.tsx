import { act } from '@testing-library/react';
import type { ReactNode } from 'react';
import type { ChangedEvent } from '../api/contracts';
import type { DataCore } from '../core';
import { createQueryClient } from '../core/query-client';
import { DataProvider } from './provider';

/** A core that has what the reads use: a query client and the change notifications. */
export function fakeCore() {
  const listeners = new Set<(event: ChangedEvent | null) => void>();
  const core = {
    queryClient: createQueryClient({ retry: false }),
    subscribeChanges: (listener: (event: ChangedEvent | null) => void) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  } as unknown as DataCore;
  const wrapper = ({ children }: { children: ReactNode }) => (
    <DataProvider adapter={{} as never} core={core}>
      {children}
    </DataProvider>
  );
  const emit = (event: ChangedEvent | null) =>
    act(async () => {
      for (const l of [...listeners]) l(event);
    });
  return { core, wrapper, emit, listeners };
}
