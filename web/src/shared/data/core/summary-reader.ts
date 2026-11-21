// Reads the Summary of a project through the query client. The query client owns the cache and
// the fetch; what stays here is what it does not know: the check of a response against the change
// notifications that arrived while it was fetched, and the presentation gate. The gate holds an
// invalidation instruction, not a fetch: while it is closed nothing is refetched, and when it
// opens one refetch is made for all that waited.
import type { QueryClient } from '@tanstack/react-query';
import type { ProjectId } from '../../api/contracts';
import type { ChangedEvent, Summary } from '../api/contracts';
import { contradictsSummary } from './change-invalidator';
import { MAX_REFETCH_ATTEMPTS, REFRESH_WINDOW_MS } from './limits';
import type { PresentationGate } from './presentation-gate';
import { SUMMARY_GC_TIME_MS } from './query-client';
import { queryKeys } from './query-keys';
import type { ResourceStore } from './resource-store';
import type { DataAdapter, SectionState, SummarySnapshot } from './types';

type Waiters = Array<() => void>;

export interface SummaryReaderOptions {
  client: QueryClient;
  adapter: DataAdapter;
  store: ResourceStore;
  gate: PresentationGate;
  /** A new Summary is in the cache. `immediate` is false for a refetch the gate could hold. */
  onCommit(projectId: ProjectId, summary: Summary, immediate: boolean): void;
  windowMs?: number;
}

/** What a query says about its data, as the state a screen shows. */
export function summaryState(query: {
  hasData: boolean;
  fetching: boolean;
  failed: boolean;
  stale: boolean;
}): SectionState {
  if (!query.hasData) {
    if (query.fetching) return 'loading';
    return query.failed ? 'error' : 'absent';
  }
  if (query.fetching) return 'refreshing';
  return query.failed || query.stale ? 'stale' : 'ready';
}

const noop = () => {};

export class SummaryReader {
  private readonly client: QueryClient;
  private readonly adapter: DataAdapter;
  private readonly store: ResourceStore;
  private readonly gate: PresentationGate;
  private readonly onCommit: SummaryReaderOptions['onCommit'];
  private readonly windowMs: number;
  /** Notifications to compare with the response of every read that is running. */
  private readonly recording = new Map<ProjectId, Set<ChangedEvent[]>>();
  /** Refetches the gate may hold, by project (counted: one may start before another ends). */
  private readonly background = new Map<ProjectId, number>();
  /** Instructions collected for one window, by project. */
  private readonly windows = new Map<
    ProjectId,
    { timer: ReturnType<typeof setTimeout>; waiters: Waiters }
  >();
  /** Instructions the gate held back, by project. */
  private readonly held = new Map<ProjectId, Waiters>();
  /** Counts the times the cache stopped being trusted (a reset); a read that began before one is old. */
  private generation = 0;
  /** The generation each running read began in, by project. */
  private readonly began = new Map<ProjectId, number>();
  private readonly lastImmediate = new Map<ProjectId, boolean>();
  private readonly offCache: () => void;
  private disposed = false;

  constructor(options: SummaryReaderOptions) {
    this.client = options.client;
    this.adapter = options.adapter;
    this.store = options.store;
    this.gate = options.gate;
    this.onCommit = options.onCommit;
    this.windowMs = options.windowMs ?? REFRESH_WINDOW_MS;
    this.offCache = this.client.getQueryCache().subscribe((event) => {
      if (event.type !== 'updated' || event.action.type !== 'success') return;
      const [kind, projectId] = event.query.queryKey as readonly [string, ProjectId];
      const data = event.query.state.data as Summary | undefined;
      if (kind === 'summary' && data)
        this.onCommit(projectId, data, this.lastImmediate.get(projectId) ?? true);
    });
  }

  /** The options of the query of a project's Summary, for `useQuery` and for a fetch by hand. */
  options(projectId: ProjectId) {
    return {
      queryKey: queryKeys.summary(projectId),
      queryFn: ({ signal }: { signal: AbortSignal }) => this.read(projectId, signal),
      gcTime: SUMMARY_GC_TIME_MS,
    };
  }

  /** What the cache holds for a project's Summary. */
  snapshot(projectId: ProjectId): SummarySnapshot {
    const state = this.client.getQueryState<Summary, Error>(queryKeys.summary(projectId));
    const data = state?.data ?? null;
    return {
      state: summaryState({
        hasData: data !== null,
        fetching: state?.fetchStatus === 'fetching',
        failed: state?.status === 'error',
        stale: state?.isInvalidated ?? false,
      }),
      data,
      version: data?.version ?? null,
      error: state?.error ?? null,
    };
  }

  /** A notification arrived: reads that are running compare their response with it. */
  record(event: ChangedEvent): void {
    for (const list of this.recording.get(event.projectId) ?? []) list.push(event);
  }

  /**
   * Reads the Summary again. `immediate` (a person asked, a decision request changed) starts now,
   * past the gate, and joins a read that is running; otherwise it is an invalidation instruction:
   * collected for a short window, and held while the gate is closed. Settles when the read ends.
   */
  load(projectId: ProjectId, immediate: boolean): Promise<void> {
    if (this.disposed) return Promise.resolve();
    if (immediate) {
      this.background.delete(projectId);
      // A person is waiting (Retry, a save): a failure shows at once, without background retries.
      return this.client
        .fetchQuery({ ...this.options(projectId), staleTime: 0, retry: false })
        .then(noop, noop);
    }
    return new Promise((resolve) => {
      const open = this.windows.get(projectId);
      if (open) {
        open.waiters.push(resolve);
        return;
      }
      const waiters = [resolve];
      const timer = setTimeout(() => {
        this.windows.delete(projectId);
        this.fire(projectId, waiters);
      }, this.windowMs);
      this.windows.set(projectId, { timer, waiters });
    });
  }

  /** Nobody trusts the Summary now; it is read again by the next instruction. */
  markStale(projectId?: ProjectId): void {
    const queryKey = projectId === undefined ? queryKeys.summaries() : queryKeys.summary(projectId);
    void this.client.invalidateQueries({ queryKey, refetchType: 'none' });
  }

  /**
   * The cache can no longer be trusted (a missed event, a reconnect, a new server run): it is marked
   * stale, and only a read that begins after this point may settle it. A read already running is
   * restarted by the next instruction, since its response may predate changes nobody was told of.
   */
  reset(): void {
    this.generation += 1;
    this.markStale();
  }

  /** The gate closed: instructions wait, and a refetch that only the gate may hold is cancelled. */
  hold(): void {
    for (const [projectId, { timer, waiters }] of this.windows) {
      clearTimeout(timer);
      this.hold1(projectId, waiters);
    }
    this.windows.clear();
    for (const [projectId, count] of this.background) {
      const key = queryKeys.summary(projectId);
      if (count <= 0 || this.client.isFetching({ queryKey: key }) === 0) continue;
      this.hold1(projectId, []);
      void this.client.cancelQueries({ queryKey: key }, { revert: true });
    }
  }

  /** The gate opened: what waited is read again once. */
  release(): void {
    const waiting = [...this.held];
    this.held.clear();
    for (const [projectId, waiters] of waiting) {
      void this.load(projectId, false).then(() => {
        for (const w of waiters) w();
      });
    }
  }

  dispose(): void {
    this.disposed = true;
    this.offCache();
    void this.client.cancelQueries({ queryKey: queryKeys.summaries() }, { revert: true });
    for (const { timer, waiters } of this.windows.values()) {
      clearTimeout(timer);
      for (const w of waiters) w();
    }
    this.windows.clear();
    for (const waiters of this.held.values()) for (const w of waiters) w();
    this.held.clear();
  }

  /** Whether the running read of a project (if any) began after the last reset. */
  private isCurrent(projectId: ProjectId): boolean {
    return (this.began.get(projectId) ?? this.generation) >= this.generation;
  }

  private hold1(projectId: ProjectId, waiters: Waiters): void {
    this.held.set(projectId, [...(this.held.get(projectId) ?? []), ...waiters]);
  }

  private fire(projectId: ProjectId, waiters: Waiters): void {
    if (this.gate.blocked) {
      this.hold1(projectId, waiters);
      return;
    }
    this.background.set(projectId, (this.background.get(projectId) ?? 0) + 1);
    const key = queryKeys.summary(projectId);
    // A read that is running is joined, not restarted: a notification that arrives while it runs is
    // compared with its response (`read`), so rapid notifications cannot starve it. One that began
    // before a reset is cancelled first, because a reset has no notification to compare.
    const cancelOld = this.isCurrent(projectId)
      ? Promise.resolve()
      : this.client.cancelQueries({ queryKey: key }, { revert: true });
    void cancelOld
      .then(() => this.client.invalidateQueries({ queryKey: key }, { cancelRefetch: false }))
      .catch(noop)
      .finally(() => {
        this.background.set(projectId, Math.max(0, (this.background.get(projectId) ?? 1) - 1));
        // Cancelled to wait for the gate: whoever waited for this read waits for the next one.
        if (this.held.has(projectId)) this.hold1(projectId, waiters);
        else for (const w of waiters) w();
      });
  }

  /**
   * One read: the response is compared with the notifications that arrived meanwhile, and read
   * again (up to `MAX_REFETCH_ATTEMPTS` in all) when it shows no sign of a change they announced.
   */
  private async read(projectId: ProjectId, signal: AbortSignal): Promise<Summary> {
    const previous = this.client.getQueryData<Summary>(queryKeys.summary(projectId)) ?? null;
    const began = this.generation;
    this.began.set(projectId, began);
    for (let attempt = 0; ; attempt += 1) {
      const recorded: ChangedEvent[] = [];
      const lists = this.recording.get(projectId) ?? new Set<ChangedEvent[]>();
      this.recording.set(projectId, lists);
      lists.add(recorded);
      let summary: Summary;
      try {
        summary = await this.adapter.fetchSummary(projectId, signal);
      } finally {
        lists.delete(recorded);
      }
      if (
        attempt + 1 < MAX_REFETCH_ATTEMPTS &&
        contradictsSummary(recorded, previous, summary, this.store)
      )
        continue;
      this.lastImmediate.set(projectId, (this.background.get(projectId) ?? 0) === 0);
      if (this.began.get(projectId) === began) this.began.delete(projectId);
      return summary;
    }
  }
}
