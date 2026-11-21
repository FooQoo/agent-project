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
  /** The list of projects. */
  projects: () => ['projects'] as const,
  /** The results of a task (they are not part of a Detail stream). */
  taskResults: (projectId: ProjectId, taskId: number) =>
    ['task-results', projectId, taskId] as const,
  /** The periodic summaries of a project's messages. */
  messageSummaries: (projectId: ProjectId) => ['message-summaries', projectId] as const,
  /** One message of a project. */
  message: (projectId: ProjectId, messageId: string) => ['message', projectId, messageId] as const,
  /** The file list of a project's Docs. */
  docsTree: (projectId: ProjectId) => ['docs-tree', projectId] as const,
  /** One file of a project's Docs. */
  docsFile: (projectId: ProjectId, path: string) => ['docs-file', projectId, path] as const,
  /** The saved notification settings. */
  notificationSettings: () => ['notification-settings'] as const,
  /** The remote-access connection status. */
  connection: () => ['connection'] as const,
  /** The devices registered for remote access, read apart from the status. */
  connectionDevices: () => ['connection-devices'] as const,
  /** The resident sessions of a project. */
  residentSessions: (projectId: ProjectId) => ['resident-sessions', projectId] as const,
  /** The tail of a resident session's log. */
  sessionLog: (projectId: ProjectId, tool: string) => ['session-log', projectId, tool] as const,
};
