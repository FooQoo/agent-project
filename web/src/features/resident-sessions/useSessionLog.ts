import type { ProjectId } from '../../shared/api/contracts';
import { queryKeys } from '../../shared/data/core/query-keys';
import { useReadQuery } from '../../shared/data/react';
import { getSessionLog, type SessionLog, type SessionTool } from './api';

/** The open dialog reads the log this often. */
export const LOG_POLL_MS = 2000;

/** What the log pane draws: the last tail read, and whether the latest read failed. */
export interface LogState {
  /** Null until the first read of the tool finishes. */
  data: SessionLog | null;
  failed: boolean;
}

/**
 * The tail of a tool's log, read from `GET /api/projects/{id}/sessions/log` every `intervalMs`
 * for as long as the caller is mounted. The query client starts the next read only after the
 * previous one ends, and reads nothing while the tab is hidden. A failed read keeps the last
 * tail and sets `failed`.
 */
export function useSessionLog(
  projectId: ProjectId,
  tool: SessionTool,
  intervalMs: number = LOG_POLL_MS,
): LogState {
  const query = useReadQuery({
    queryKey: queryKeys.sessionLog(projectId, tool),
    queryFn: () => getSessionLog(projectId, tool),
    refetchIntervalMs: intervalMs,
  });
  // An unchanged tail keeps the same object (structural sharing), so the pane does not render again.
  return { data: query.data ?? null, failed: query.isError };
}
