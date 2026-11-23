import type { ColumnName } from '../../../shared/api/contracts';

/**
 * The cards the board shows for a version filter. `null` ("All") shows every card. A version keeps
 * the cards planned for it and the unplanned ones in the Todo, Doing and Done lanes; the Backlog is
 * never filtered.
 */
export function filterByVersion<
  T extends { taskStatus: ColumnName; plannedVersion?: string | null },
>(tasks: readonly T[], filter: string | null): T[] {
  if (filter === null) return [...tasks];
  return tasks.filter(
    (task) =>
      task.taskStatus === 'backlog' ||
      task.plannedVersion === undefined ||
      task.plannedVersion === null ||
      task.plannedVersion === filter,
  );
}
