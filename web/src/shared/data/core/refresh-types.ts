// Shared by the scheduler and the fetches it starts.
import type { ProjectId } from '../../api/contracts';
import type { ChangedEvent, DetailSection } from '../api/contracts';
import type { ChangeInvalidator } from './change-invalidator';
import { keyOf, type ResourceStore } from './resource-store';
import { StreamRejected } from './stream-reader';
import type { SummaryReader } from './summary-reader';
import type { DataAdapter, ResourceKey, SummarySnapshot } from './types';

export { keyOf };

export type Waiters = Array<() => void>;

export interface Item {
  id: string;
  projectId: ProjectId;
  key: ResourceKey;
  sections: Set<DetailSection>;
  immediate: boolean;
  /** The window it was queued in has passed. */
  ready: boolean;
  attempt: number;
  waiters: Waiters;
  order: number;
}

export interface Flight {
  id: string;
  projectId: ProjectId;
  key: ResourceKey;
  sections: DetailSection[];
  immediate: boolean;
  attempt: number;
  controller: AbortController;
  waiters: Waiters;
  /** A newer notification outdated it: do not keep the response, fetch again. */
  superseded: boolean;
  /** Stopped to hold the background back: fetch again later. */
  requeue: boolean;
  /** The resource is gone: just stop. */
  cancelled: boolean;
}

export const detailId = (key: ResourceKey) => `d:${keyOf(key)}`;
export const toError = (e: unknown) => (e instanceof Error ? e : new Error(String(e)));
export const aborted = () => new StreamRejected('aborted', 'outdated');

export interface EnqueueRequest {
  projectId: ProjectId;
  key: ResourceKey;
  sections: Iterable<DetailSection>;
  immediate: boolean;
  waiters: Waiters;
  attempt?: number;
}

/** What a fetch needs from the scheduler that started it. */
export interface FlightHost {
  readonly adapter: DataAdapter;
  readonly store: ResourceStore;
  /** What the query client holds for a project's Summary. */
  getSummary(projectId: ProjectId): SummarySnapshot;
  /** Sections a notification outdated; the Summary does not yet say what version they have. */
  readonly unverified: Set<string>;
  readonly disposed: boolean;
  enqueue(request: EnqueueRequest): void;
  loadSummary(projectId: ProjectId, options?: { immediate?: boolean }): Promise<void>;
  hasFlight(id: string): boolean;
  cancelDetail(key: ResourceKey): void;
}

export type { ChangedEvent };

/** What the actions on a cache invalidation need from the scheduler. */
export interface SchedulerHost extends FlightHost {
  readonly flights: Map<string, Flight>;
  readonly queue: Map<string, Item>;
  readonly summary: SummaryReader;
  readonly invalidator: ChangeInvalidator;
  watched(): Iterable<ProjectId>;
}
