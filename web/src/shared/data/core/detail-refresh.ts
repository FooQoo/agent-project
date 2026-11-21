import type { QueryClient, QueryFunction, QueryKey } from '@tanstack/react-query';
import type { DetailSection } from '../api/contracts';
import { MAX_REFETCH_ATTEMPTS } from './limits';

export interface RefreshSteps {
  disposed(): boolean;
  /** Reads the sections now; rejects with a failed read. */
  read(sections: readonly DetailSection[]): Promise<void>;
  /** The sections that hold another version than the trusted Summary names. */
  outdated(): readonly DetailSection[];
}

/**
 * Reads until what is held is what the Summary names (up to `MAX_REFETCH_ATTEMPTS` reads): a read
 * that was joined may be of a version the Summary has since replaced. Rejects when it cannot match.
 */
export async function refreshUntilCurrent(
  sections: readonly DetailSection[],
  steps: RefreshSteps,
): Promise<void> {
  let pending = sections;
  for (let attempt = 0; attempt < MAX_REFETCH_ATTEMPTS && pending.length > 0; attempt += 1) {
    if (steps.disposed()) return;
    await steps.read(pending);
    pending = steps.outdated();
  }
  if (pending.length > 0) throw new Error('The Summary names a newer version than the one read');
}

/** Fetches queries now (staleTime 0, no retries, joining a read that runs); rejects with the first failure. */
export async function fetchNow<Q extends { queryKey: QueryKey; queryFn: QueryFunction<unknown> }>(
  client: QueryClient,
  queries: readonly Q[],
  onEach: (query: Q) => void,
): Promise<void> {
  const results = await Promise.allSettled(
    queries.map((query) => {
      onEach(query);
      return client.fetchQuery({ ...query, staleTime: 0, retry: false });
    }),
  );
  const failed = results.find((result) => result.status === 'rejected');
  if (failed) throw failed.reason;
}
