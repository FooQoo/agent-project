// Gives a React tree one data core: the store, the adapter's connection and the presentation gate.
import { QueryClientProvider } from '@tanstack/react-query';
import { createContext, type ReactNode, useContext, useEffect, useMemo, useRef } from 'react';
import { DataCore, type DataCoreOptions } from '../core/data-core';

const Context = createContext<DataCore | null>(null);

export interface DataProviderProps extends DataCoreOptions {
  children: ReactNode;
  /** A core made by the caller; the provider then neither makes nor starts one. */
  core?: DataCore;
}

export function DataProvider({ children, core: given, ...options }: DataProviderProps) {
  // The callbacks may be new on every render; the core calls the latest ones.
  const latest = useRef(options);
  latest.current = options;
  const { adapter, visibility, queryClient } = options;
  const core = useMemo(
    () =>
      given ??
      new DataCore({
        adapter,
        visibility,
        queryClient,
        onReady: (build) => latest.current.onReady?.(build),
        onPreferences: (payload) => latest.current.onPreferences?.(payload),
        onMode: (payload) => latest.current.onMode?.(payload),
        onConnectionState: (state) => latest.current.onConnectionState?.(state),
      }),
    [given, adapter, visibility, queryClient],
  );
  useEffect(() => {
    if (given) return undefined;
    core.start();
    return () => core.stop();
  }, [core, given]);
  return (
    <QueryClientProvider client={core.queryClient}>
      <Context.Provider value={core}>{children}</Context.Provider>
    </QueryClientProvider>
  );
}

export function useDataCore(): DataCore {
  const core = useContext(Context);
  if (!core) throw new Error('useDataCore needs a DataProvider above it');
  return core;
}

/** The core above, or null where there is no provider (a Story, a test of a single part). */
export function useOptionalDataCore(): DataCore | null {
  return useContext(Context);
}
