// The read of one screen (a list, a dialog) through the query client. Nothing is read again on its
// own: a change notification that concerns the query marks it stale, and a read that a person asks
// for (Retry, a refresh) joins one that is running. A read that has a Retry button does not retry
// in the background, so that its failure shows at once. Where there is no data provider (a Story,
// a test of one part) the read uses a client of its own, kept for as long as the screen is.
import {
  type QueryClient,
  type QueryKey,
  type UseQueryResult,
  useQuery,
} from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import type { ChangedEvent } from '../api/contracts';
import { createQueryClient } from '../core/query-client';
import { useOptionalDataCore } from './provider';

export interface ReadQueryOptions<T> {
  queryKey: QueryKey;
  queryFn: () => Promise<T>;
  enabled?: boolean;
  /** Whether a change notification makes this read stale. Without it only a reset (a lost stream) does. */
  changedBy?: (event: ChangedEvent) => boolean;
  /** Reads again this often while the tab is shown (a log that is followed). */
  refetchIntervalMs?: number;
  /** Keep the data this long after the last screen left. Zero (the default) reads again on opening. */
  gcTimeMs?: number;
}

/** The client a read uses: the data core's, or one of its own where there is no core. */
export function useReadClient(): QueryClient {
  const core = useOptionalDataCore();
  const [own] = useState(() => (core ? null : createQueryClient({ retry: false })));
  return core?.queryClient ?? (own as QueryClient);
}

export function useReadQuery<T>(
  options: ReadQueryOptions<T>,
  /** The client from `useReadClient`, when the caller also needs it (a quiet refresh). */
  given?: QueryClient,
): UseQueryResult<T, Error> {
  const core = useOptionalDataCore();
  const own = useReadClient();
  const client = given ?? own;
  const { queryKey } = options;
  const hash = JSON.stringify(queryKey);
  const changedBy = useRef(options.changedBy);
  changedBy.current = options.changedBy;
  // biome-ignore lint/correctness/useExhaustiveDependencies: the key is compared by its hash.
  useEffect(() => {
    if (!core) return undefined;
    return core.subscribeChanges((event) => {
      // `null`: the stream can no longer be trusted. A read that is running is joined.
      if (event === null || changedBy.current?.(event) === true)
        void client.invalidateQueries({ queryKey, exact: true }, { cancelRefetch: false });
    });
  }, [core, client, hash]);
  return useQuery<T, Error>(
    {
      queryKey,
      queryFn: () => options.queryFn(),
      enabled: options.enabled ?? true,
      gcTime: options.gcTimeMs ?? 0,
      retry: false,
      refetchInterval: options.refetchIntervalMs ?? false,
    },
    client,
  );
}
