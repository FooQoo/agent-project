// The query client of the web app. Data is fresh until a change notification says otherwise, so
// nothing is read again on its own: not on a refocus, not on a reconnect of the network (the
// event stream reopening and sending `ready` is the signal), and not by age.
import { QueryClient } from '@tanstack/react-query';
import { httpStatusOf } from '../../api/httpStatus';

/** How long a query that no screen shows stays in the cache. */
export const GC_TIME_MS = 5 * 60_000;
/** The same for a Summary, which is the most expensive to read again. */
export const SUMMARY_GC_TIME_MS = 30 * 60_000;
/** A failed read is tried again this many times. */
export const READ_RETRIES = 2;

/** An answer of 4xx says the request is wrong; asking again would only say it again. */
function isClientError(error: unknown): boolean {
  const status = httpStatusOf(error);
  return status !== null && status >= 400 && status < 500;
}

export interface QueryClientOptions {
  /** Whether a failed read is tried again. Off by default in unit tests, which would wait for it. */
  retry?: boolean;
}

export function createQueryClient({
  retry = import.meta.env.MODE !== 'test',
}: QueryClientOptions = {}): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: Number.POSITIVE_INFINITY,
        gcTime: GC_TIME_MS,
        refetchOnWindowFocus: false,
        refetchOnReconnect: false,
        networkMode: 'always',
        retry: retry ? (count, error) => count < READ_RETRIES && !isClientError(error) : false,
      },
      mutations: { networkMode: 'always', retry: false },
    },
  });
}
