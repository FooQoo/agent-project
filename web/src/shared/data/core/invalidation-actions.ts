// What the scheduler does with a cache invalidation: ask for a new Summary, and mark the Details
// as not to be trusted when the whole cache cannot be. A Detail is read again by the new Summary
// (`DetailReader.reconcile`), never by the notification itself.
import type { ProjectId } from '../../api/contracts';
import type { ChangeInvalidator, Invalidation } from './change-invalidator';
import type { DetailReader } from './detail-reader';
import type { SummaryReader } from './summary-reader';
import type { ResetReason } from './types';

/** What the actions need from the scheduler. */
export interface SchedulerHost {
  readonly disposed: boolean;
  readonly summary: SummaryReader;
  readonly details: DetailReader;
  readonly invalidator: ChangeInvalidator;
  watched(): Iterable<ProjectId>;
  loadSummary(projectId: ProjectId, options?: { immediate?: boolean }): Promise<void>;
}

/** The cache can no longer be trusted: everything is stale until a new Summary says otherwise. */
export function resetCache(host: SchedulerHost, reason: ResetReason): void {
  if (host.disposed) return;
  host.invalidator.reset(reason);
  host.summary.reset();
  host.details.reset();
  for (const projectId of host.watched()) void host.loadSummary(projectId, { immediate: false });
}

/** `immediate` reads at once, past the gate, as for a decision request being asked or answered. */
export function applyInvalidation(
  host: SchedulerHost,
  projectId: ProjectId,
  inv: Invalidation,
  immediate = false,
): void {
  if (host.disposed) return;
  if (inv.summary) void host.loadSummary(projectId, { immediate });
}

/** Something opened: background reads stop and wait for the gate. */
export function holdBackground(host: SchedulerHost): void {
  host.summary.hold();
  host.details.hold();
}
