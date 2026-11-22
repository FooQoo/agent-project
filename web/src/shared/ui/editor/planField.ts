import type { ReactNode } from 'react';

/** The change a field of an editor makes when the editor saves; it rejects when the change fails. */
export type PendingSave = () => Promise<void>;

/**
 * What an editor hands to a field the page supplies (the planned version): whether it can be edited
 * now, and where to report its pending change (`null` when the person has changed nothing). The
 * editor runs the pending change as the last step of its own save.
 */
export interface PlanFieldArgs {
  disabled: boolean;
  onPending: (pending: PendingSave | null) => void;
}

export type RenderPlanField = (args: PlanFieldArgs) => ReactNode;
