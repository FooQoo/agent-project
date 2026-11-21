// The keys of the query client's cache, built in one place. A later read of a task or an epic
// takes its key from here too, so that one key names one thing everywhere.
import type { ProjectId } from '../../api/contracts';

export const queryKeys = {
  /** Every Summary, of every project. */
  summaries: () => ['summary'] as const,
  /** The Summary of a project: the board's list of tasks and epics. */
  summary: (projectId: ProjectId) => ['summary', projectId] as const,
};
