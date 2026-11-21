// Reads the Details of tasks and epics through the query client, one query per section. The
// stream itself stays hand-written (`readDetail` checks it and keeps only a whole version); the
// query function consumes it. What the query client does not know stays here: when a Detail is
// out of date (the Summary names another version of it), the presentation gate, and a reset of
// the cache, after which only a read that began later may settle a Detail.
import {
  hashKey,
  notifyManager,
  type Query,
  type QueryCacheNotifyEvent,
  type QueryClient,
} from '@tanstack/react-query';
import type { ProjectId } from '../../api/contracts';
import type { DetailSection, DetailVersions, Summary } from '../api/contracts';
import { DetailBatch } from './detail-batch';
import { DetailPartials } from './detail-partials';
import type { DetailEntry } from './detail-snapshot';
import { MAX_REFETCH_ATTEMPTS } from './limits';
import type { PresentationGate } from './presentation-gate';
import { queryKeys } from './query-keys';
import { StreamRejected } from './stream-reader';
import type { DataAdapter, ResourceKey } from './types';

const noop = () => {};
const KINDS = ['task', 'epic'] as const;

export interface DetailReaderOptions {
  client: QueryClient;
  adapter: DataAdapter;
  gate: PresentationGate;
  /** Reads the Summary of a project again now, past the gate. */
  loadSummary(projectId: ProjectId): Promise<void>;
}

function keyOfQuery(query: Query): { key: ResourceKey; section: DetailSection } {
  const [kind, projectId, id, section] = query.queryKey as [
    ResourceKey['kind'],
    ProjectId,
    number | string,
    DetailSection,
  ];
  return { key: { kind, projectId, id } as ResourceKey, section };
}

function entryOf(summary: Summary, key: ResourceKey) {
  return key.kind === 'task'
    ? summary.tasks.find((t) => t.id === key.id)
    : summary.epics.find((e) => e.id === key.id);
}

export class DetailReader {
  readonly partials = new DetailPartials();
  private readonly client: QueryClient;
  private readonly batch: DetailBatch;
  private readonly gate: PresentationGate;
  private readonly loadSummary: DetailReaderOptions['loadSummary'];
  /** Counts the times the cache stopped being trusted (a reset); a read that began before one is old. */
  private generation = 0;
  private readonly began = new Map<string, number>();
  /** Reads that only the gate may hold, by query hash: a person is not waiting for them. */
  private readonly background = new Set<string>();
  /** Projects whose Details the gate kept from being read; compared again when it opens. */
  private readonly held = new Set<ProjectId>();
  private readonly offCache: () => void;
  private disposed = false;

  constructor(options: DetailReaderOptions) {
    this.client = options.client;
    this.batch = new DetailBatch({
      adapter: options.adapter,
      expected: (key, sections) => this.expected(key, sections),
      onPartial: (key, section, data) =>
        this.partials.set(hashKey(queryKeys.detail(key, section)), data),
    });
    this.gate = options.gate;
    this.loadSummary = options.loadSummary;
    this.offCache = this.client.getQueryCache().subscribe((event) => {
      if (event.type === 'removed' || event.type === 'updated') this.watch(event);
    });
  }

  /**
   * A detail query changed. When its read ended, what it had received so far is dropped, after the
   * screens that show the query have been told (or they would show nothing between the two); and
   * a whole version is compared with the Summary at once, since the read may have begun before the
   * Summary named a newer version.
   */
  private watch(event: QueryCacheNotifyEvent): void {
    const { query } = event;
    if (!(KINDS as readonly string[]).includes(query.queryKey[0] as string)) return;
    if (event.type === 'removed' || query.state.fetchStatus === 'idle') {
      notifyManager.schedule(() => this.partials.clear(query.queryHash));
    }
    if (event.type !== 'updated' || event.action.type !== 'success') return;
    const summary = this.trusted(keyOfQuery(query).key.projectId);
    if (summary) this.settle(query, summary, this.gate.blocked, true);
  }

  /** The options of the query of one section, for `useQuery` and for a fetch by hand. */
  options<S extends DetailSection>(key: ResourceKey, section: S) {
    return {
      queryKey: queryKeys.detail(key, section),
      queryFn: ({ signal }: { signal: AbortSignal }) => this.read<S>(key, section, signal),
    };
  }

  /** The whole version a section holds, or null. */
  entry<S extends DetailSection>(key: ResourceKey, section: S): DetailEntry<S> | null {
    return this.client.getQueryData<DetailEntry<S>>(queryKeys.detail(key, section)) ?? null;
  }

  /** Whether any query exists for the section. */
  has(key: ResourceKey, section: DetailSection): boolean {
    return (
      this.client.getQueryCache().find({ queryKey: queryKeys.detail(key, section) }) !== undefined
    );
  }

  /** The version of a section the cache holds, if any. */
  versionOf(key: ResourceKey, section: DetailSection): string | null {
    return this.entry(key, section)?.version ?? null;
  }

  /**
   * Reads sections again now, past the gate and joining a read that is running. A person waits
   * (Retry, an editor opening): a failure shows at once, without background retries. Settles when
   * the reads end; what failed is in the query.
   */
  async fetch(key: ResourceKey, sections: readonly DetailSection[]): Promise<void> {
    if (this.disposed) return;
    await Promise.all(
      sections.map((section) => {
        const options = this.options(key, section);
        this.background.delete(hashKey(options.queryKey));
        return this.client.fetchQuery({ ...options, staleTime: 0, retry: false }).then(noop, noop);
      }),
    );
  }

  /**
   * A screen opens a Detail: what it shows is compared with the Summary now, and what the Summary
   * names a newer version of is read at once. Reads that run for it are no longer held by the gate.
   */
  open(key: ResourceKey, sections: readonly DetailSection[]): void {
    const summary = this.trusted(key.projectId);
    for (const section of sections) {
      const queryKey = queryKeys.detail(key, section);
      this.background.delete(hashKey(queryKey));
      const query = this.client.getQueryCache().find({ queryKey });
      if (query && summary) this.settle(query, summary, false, false);
    }
  }

  /** A new Summary is in the cache: the Details it names other versions of are read again. */
  reconcile(projectId: ProjectId, summary: Summary, immediate: boolean): void {
    if (this.disposed) return;
    const gated = !immediate && this.gate.blocked;
    for (const kind of KINDS) {
      const queries = this.client
        .getQueryCache()
        .findAll({ queryKey: queryKeys.projectDetails(projectId, kind) });
      for (const query of queries) this.settle(query, summary, gated, !immediate);
    }
  }

  /** Nobody trusts the Details of a project (or of any) now; each is checked by the next Summary. */
  markStale(projectId?: ProjectId): void {
    for (const kind of KINDS) {
      const queryKey = projectId === undefined ? [kind] : queryKeys.projectDetails(projectId, kind);
      void this.client.invalidateQueries({ queryKey, refetchType: 'none' });
    }
  }

  /** The cache can no longer be trusted: a read already running may predate changes nobody told of. */
  reset(): void {
    this.generation += 1;
    this.markStale();
  }

  /** The gate closed: reads it may hold stop, and are made again when it opens. */
  hold(): void {
    for (const hash of this.background) {
      const query = this.client.getQueryCache().get(hash);
      if (!query || query.state.fetchStatus !== 'fetching') continue;
      this.held.add(keyOfQuery(query).key.projectId);
      void query.cancel({ revert: true }).catch(noop);
    }
    this.background.clear();
  }

  /** The gate opened: the Details that waited are compared with the Summary again. */
  release(): void {
    const projects = [...this.held];
    this.held.clear();
    for (const projectId of projects) {
      const summary = this.trusted(projectId);
      if (summary) this.reconcile(projectId, summary, false);
    }
  }

  dispose(): void {
    this.disposed = true;
    this.offCache();
    for (const kind of KINDS)
      void this.client.cancelQueries({ queryKey: [kind] }, { revert: true });
    this.background.clear();
    this.held.clear();
  }

  /** The Summary of a project, unless it is marked as not to be trusted. */
  private trusted(projectId: ProjectId): Summary | null {
    const state = this.client.getQueryState<Summary>(queryKeys.summary(projectId));
    return state?.data && !state.isInvalidated ? state.data : null;
  }

  /** Whether the running read of a query (if any) began before the last reset. */
  private isOutdated(query: Query): boolean {
    return (
      query.state.fetchStatus === 'fetching' &&
      (this.began.get(query.queryHash) ?? this.generation) < this.generation
    );
  }

  /**
   * Compares one query with the Summary. A whole version that matches is trusted again; one that
   * differs is read again, past the gate unless the gate is closed (then it waits, marked as out of
   * date). A first read, or one that failed, is not touched: it asks the Summary itself.
   */
  private settle(query: Query, summary: Summary, gated: boolean, background: boolean): void {
    const { key, section } = keyOfQuery(query);
    const entry = entryOf(summary, key);
    if (!entry) {
      // The Summary does not list it: a card that is gone, or one it never lists (an archived
      // one). What a screen shows stays; what nothing shows is dropped.
      if (query.getObserversCount() === 0)
        this.client.removeQueries({ queryKey: query.queryKey, exact: true });
      return;
    }
    const expected = (entry.detailVersions as DetailVersions)[section];
    const have = (query.state.data as DetailEntry | undefined)?.version;
    if (expected === undefined) return;
    if (have === expected) {
      if (query.state.isInvalidated) {
        // The same version: trusted again. Not a fetch, so the read that may be running goes on.
        this.client.setQueryData(query.queryKey, query.state.data);
      }
      return;
    }
    const outdated = this.isOutdated(query);
    if (have === undefined && !outdated) return;
    const exact = { queryKey: query.queryKey, exact: true } as const;
    if (gated) {
      this.held.add(key.projectId);
      void this.client.invalidateQueries({ ...exact, refetchType: 'none' });
      return;
    }
    if (background && query.getObserversCount() > 0) this.background.add(query.queryHash);
    // A read that began before a reset is cancelled first, because a reset has no notification to
    // compare; one that is current is joined, so rapid Summaries cannot starve it.
    const cancelOld = outdated
      ? this.client.cancelQueries(exact, { revert: true })
      : Promise.resolve();
    void cancelOld
      .then(() => this.client.invalidateQueries(exact, { cancelRefetch: false }))
      .catch(noop)
      .finally(() => this.background.delete(query.queryHash));
  }

  /**
   * One read: the stream must have the version the Summary names (when it can be trusted). When
   * the server has another, the Summary is older than the server: it is read again, then this
   * (up to `MAX_REFETCH_ATTEMPTS` in all).
   */
  private async read<S extends DetailSection>(
    key: ResourceKey,
    section: S,
    signal: AbortSignal,
  ): Promise<DetailEntry<S>> {
    const hash = hashKey(queryKeys.detail(key, section));
    const began = this.generation;
    this.began.set(hash, began);
    this.partials.clear(hash);
    try {
      for (let attempt = 0; ; attempt += 1) {
        try {
          return await this.batch.read(key, section, signal);
        } catch (error) {
          const mismatch = error instanceof StreamRejected && error.reason === 'version-mismatch';
          if (!mismatch || attempt + 1 >= MAX_REFETCH_ATTEMPTS) throw error;
          this.partials.clear(hash);
          await this.loadSummary(key.projectId);
        }
      }
    } finally {
      if (this.began.get(hash) === began) this.began.delete(hash);
    }
  }

  /** The versions the Summary names, unless it is marked as not to be trusted. */
  private expected(key: ResourceKey, sections: DetailSection[]): DetailVersions | undefined {
    const summary = this.trusted(key.projectId);
    const named = summary
      ? (entryOf(summary, key)?.detailVersions as DetailVersions | undefined)
      : undefined;
    if (!named) return undefined;
    const versions: DetailVersions = {};
    for (const section of sections)
      if (named[section] !== undefined) versions[section] = named[section];
    return versions;
  }
}
