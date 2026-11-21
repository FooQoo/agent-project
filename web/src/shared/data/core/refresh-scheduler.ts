// Decides when the data core goes to the server again. It checks the sequence of the change
// notifications, passes them to the Summary reader (which compares a response with the changes
// that arrived while it was read), and holds what the presentation gate holds. It asks for a new
// Summary when a change calls for one; the Summary then names the Details to read again
// (`DetailReader`), so no change reads a Detail by itself.
import type { ProjectId } from '../../api/contracts';
import type { ChangedEvent } from '../api/contracts';
import { type ChangeInvalidator, changesRequest } from './change-invalidator';
import type { DetailReader } from './detail-reader';
import {
  applyInvalidation,
  holdBackground,
  resetCache,
  type SchedulerHost,
} from './invalidation-actions';
import type { PresentationGate } from './presentation-gate';
import type { SummaryReader } from './summary-reader';
import type { ResetReason, SummarySnapshot } from './types';

export interface SchedulerOptions {
  gate: PresentationGate;
  invalidator: ChangeInvalidator;
  summary: SummaryReader;
  details: DetailReader;
  /** The projects whose Summary the screens are showing. */
  watchedProjects(): Iterable<ProjectId>;
}

export interface LoadOptions {
  /**
   * A person asked for it (opening, refresh, save): it starts at once and is not held back by
   * the gate. Otherwise it is a background update.
   */
  immediate?: boolean;
}

export class RefreshScheduler implements SchedulerHost {
  private readonly gate: PresentationGate;
  readonly invalidator: ChangeInvalidator;
  readonly summary: SummaryReader;
  readonly details: DetailReader;
  readonly watched: () => Iterable<ProjectId>;
  /** Read by the actions, which stop quietly once it is true. */
  disposed = false;
  private readonly offGate: () => void;

  constructor(options: SchedulerOptions) {
    this.gate = options.gate;
    this.invalidator = options.invalidator;
    this.summary = options.summary;
    this.details = options.details;
    this.watched = options.watchedProjects;
    this.offGate = this.gate.subscribe((event) => {
      if (event === 'held') holdBackground(this);
      else {
        this.summary.release();
        this.details.release();
      }
    });
  }

  /**
   * Reads the Summary of a project again, or joins the read that is running. A background one
   * (not `immediate`) is an invalidation instruction: it waits for its window and for the gate.
   */
  loadSummary(projectId: ProjectId, options: LoadOptions = {}): Promise<void> {
    return this.summary.load(projectId, options.immediate ?? true);
  }

  getSummary(projectId: ProjectId): SummarySnapshot {
    return this.summary.snapshot(projectId);
  }

  /**
   * A `changed` event. The sequence is checked now; what it changes waits for the gate, except a
   * decision request being asked or answered, which is read and shown at once.
   */
  onChanged(event: ChangedEvent): void {
    const reason = this.invalidator.observe(event);
    this.summary.record(event);
    if (reason) {
      this.gate.submit(() => resetCache(this, reason), { key: 'reset' });
    } else {
      const urgent = changesRequest(event);
      this.gate.submit(
        () => applyInvalidation(this, event.projectId, this.invalidator.apply(event), urgent),
        { immediate: urgent },
      );
    }
  }

  /** The adapter says the cache can no longer be trusted. */
  onReset(reason: ResetReason): void {
    this.gate.submit(() => resetCache(this, reason), { key: 'reset' });
  }

  /**
   * The subscription is open. The first Summary is read by the screen's query once this is so; a
   * reconnect means events may have been missed, so the cache is read again.
   */
  connected(reconnect: boolean): void {
    if (!reconnect) {
      // A Summary read before the first subscription (a page that started hidden, or with the
      // stream down) may have missed a change: read it again. Normally none is held yet.
      for (const projectId of this.watched()) {
        if (this.summary.snapshot(projectId).data === null) continue;
        this.gate.submit(
          () => {
            this.summary.reset();
            this.details.reset();
            void this.loadSummary(projectId, { immediate: false });
          },
          { key: `first:${projectId}` },
        );
      }
      return;
    }
    this.invalidator.forgetSequence();
    this.gate.submit(() => resetCache(this, 'reconnected'), { key: 'reset' });
  }

  dispose(): void {
    this.disposed = true;
    this.offGate();
    this.summary.dispose();
    this.details.dispose();
  }
}
