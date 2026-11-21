// The data core as one object: the readers, the gate, the scheduler and the connection, wired
// together. A framework binding (React) needs only this and an adapter.
import type { QueryClient } from '@tanstack/react-query';
import type { ProjectId } from '../../api/contracts';
import type { ChangedEvent } from '../api/contracts';
import { ChangeInvalidator } from './change-invalidator';
import { DetailReader } from './detail-reader';
import { type ConnectionState, EventConnection, type VisibilitySource } from './event-connection';
import { PresentationGate } from './presentation-gate';
import { createQueryClient } from './query-client';
import { RefreshScheduler } from './refresh-scheduler';
import { SummaryReader } from './summary-reader';
import type { DataAdapter, SummarySnapshot } from './types';

export interface DataCoreOptions {
  adapter: DataAdapter;
  /** The query client that holds the Summaries and the Details; one is made when none is given. */
  queryClient?: QueryClient;
  visibility?: VisibilitySource;
  /** The build ID of the server's `ready` event. */
  onReady?(build: string): void;
  onPreferences?(payload: unknown): void;
  onConnectionState?(state: ConnectionState): void;
}

export class DataCore {
  readonly queryClient: QueryClient;
  readonly summary: SummaryReader;
  readonly details: DetailReader;
  readonly gate: PresentationGate;
  readonly scheduler: RefreshScheduler;
  readonly connection: EventConnection;
  private readonly watched = new Map<ProjectId, number>();
  private readonly changeListeners = new Set<(event: ChangedEvent | null) => void>();
  /** Projects whose last screen left: they read as stale again when watched, whatever a cancelled read restored. */
  private readonly left = new Set<ProjectId>();
  private readonly stateListeners = new Set<() => void>();

  constructor(options: DataCoreOptions) {
    this.gate = new PresentationGate();
    this.queryClient = options.queryClient ?? createQueryClient();
    this.details = new DetailReader({
      client: this.queryClient,
      adapter: options.adapter,
      gate: this.gate,
      loadSummary: (projectId) => this.summary.load(projectId, true),
    });
    this.summary = new SummaryReader({
      client: this.queryClient,
      adapter: options.adapter,
      versionOf: (key, section) => this.details.versionOf(key, section),
      gate: this.gate,
      // The Details the new Summary names other versions of are read again.
      onCommit: (projectId, summary, immediate) =>
        this.details.reconcile(projectId, summary, immediate),
    });
    const invalidator = new ChangeInvalidator({
      isActiveProject: (id) => this.watched.has(id),
      hasDetail: (key, section) => this.details.has(key, section),
    });
    this.scheduler = new RefreshScheduler({
      gate: this.gate,
      invalidator,
      summary: this.summary,
      details: this.details,
      watchedProjects: () => [...this.watched.keys()],
    });
    this.connection = new EventConnection({
      adapter: options.adapter,
      visibility: options.visibility,
      onChanged: (event) => {
        this.scheduler.onChanged(event);
        for (const listener of [...this.changeListeners]) listener(event);
      },
      onReset: (reason) => {
        this.scheduler.onReset(reason);
        for (const listener of [...this.changeListeners]) listener(null);
      },
      onConnected: (reconnect) => {
        this.scheduler.connected(reconnect);
        // Events may have been missed: data read by its own route must be read again.
        if (reconnect) for (const listener of [...this.changeListeners]) listener(null);
      },
      onReady: options.onReady,
      onPreferences: options.onPreferences,
      onState: (state) => {
        options.onConnectionState?.(state);
        for (const listener of [...this.stateListeners]) listener();
      },
    });
  }

  /**
   * Follows every change notification as it arrives, for data that is read by its own route and
   * not through the Summary. The listener gets `null` when the cache can no longer be trusted (a
   * missed event, a new connection), meaning "read it again". Returns the function that ends it.
   */
  subscribeChanges(listener: (event: ChangedEvent | null) => void): () => void {
    this.changeListeners.add(listener);
    return () => {
      this.changeListeners.delete(listener);
    };
  }

  /** Follows the state of the connection (for `useSyncExternalStore`). */
  subscribeState = (listener: () => void): (() => void) => {
    this.stateListeners.add(listener);
    return () => {
      this.stateListeners.delete(listener);
    };
  };

  /**
   * Whether a screen may read the Summary now. The first read waits for the subscription (the
   * `ready` event), so that no change falls between the read and the subscription. When the
   * connection is down or the tab is hidden nothing else will read it, so it is read at once.
   */
  get canReadSummary(): boolean {
    const state = this.connection.state;
    return state !== 'connecting' && state !== 'stopped';
  }

  /** Opens the connection. Summaries wanted before this wait for the subscription. */
  start(): void {
    this.connection.start();
  }

  /** Closes the connection; `start` opens it again. The cache stays. */
  stop(): void {
    this.connection.stop();
  }

  dispose(): void {
    this.connection.stop();
    this.scheduler.dispose();
    this.gate.dispose();
  }

  /**
   * A screen shows this project's Summary (read by its query). Its change notifications are
   * followed while at least one screen watches. Returns the function that ends the watch.
   */
  watchProject(projectId: ProjectId): () => void {
    this.watched.set(projectId, (this.watched.get(projectId) ?? 0) + 1);
    // A read cancelled when the screen left restores the state from before it, without the stale mark.
    if (this.left.delete(projectId)) {
      this.summary.markStale(projectId);
      this.details.markStale(projectId);
    }
    let ended = false;
    return () => {
      if (ended) return;
      ended = true;
      const left = (this.watched.get(projectId) ?? 1) - 1;
      if (left > 0) {
        this.watched.set(projectId, left);
        return;
      }
      this.watched.delete(projectId);
      this.left.add(projectId);
      this.markProjectStale(projectId);
    };
  }

  /** What the query client holds for a project's Summary. */
  getSummary(projectId: ProjectId): SummarySnapshot {
    return this.summary.snapshot(projectId);
  }

  /** A person asked for the Summary again (refresh, after a save or an answer). */
  refreshSummary(projectId: ProjectId): Promise<void> {
    return this.scheduler.loadSummary(projectId, { immediate: true });
  }

  /** Nobody follows this project's notifications now, so what it holds may be out of date. */
  private markProjectStale(projectId: ProjectId): void {
    this.summary.markStale(projectId);
    this.details.markStale(projectId);
  }
}
