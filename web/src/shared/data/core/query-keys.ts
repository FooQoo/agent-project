// The keys of the query client's cache, built in one place, so that one key names one thing
// everywhere. A Detail is one query per section: a task has a `document`, a `history` and
// `results` key, an epic only a `document` key.
import type { ProjectId } from '../../api/contracts';
import type { DetailSection } from '../api/contracts';
import type { ResourceKey } from './types';

export const queryKeys = {
  /** Every Summary, of every project. */
  summaries: () => ['summary'] as const,
  /** The Summary of a project: the board's list of tasks and epics. */
  summary: (projectId: ProjectId) => ['summary', projectId] as const,
  /** One section of a task's or an epic's Detail. */
  detail: (key: ResourceKey, section: DetailSection) =>
    [key.kind, key.projectId, key.id, section] as const,
  /** Every Detail of a task or an epic of a project. */
  projectDetails: (projectId: ProjectId, kind: ResourceKey['kind']) => [kind, projectId] as const,
};
