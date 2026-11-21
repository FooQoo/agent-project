import { useCallback, useEffect } from 'react';
import { getJson, projectPath } from '../../../shared/api/http';
import type { ProjectId, TaskResult } from '../../../shared/api/contracts';
import { queryKeys } from '../../../shared/data/core/query-keys';
import { useReadQuery } from '../../../shared/data/react';

export type TaskResultsState =
  | { status: 'loading' }
  | { status: 'error' }
  | { status: 'ready'; results: readonly TaskResult[] };

/**
 * Reads a task's results. The Summary and Detail streams do not carry them, so they are read from
 * the results route, and read only while `enabled` (the tab is shown), and read again when
 * `refreshKey` changes (the number of history entries: each delivery adds one) or the tab is shown
 * again. A failed refresh keeps what was shown and says so in `refreshFailed`.
 */
export function useTaskResults(
  projectId: ProjectId,
  taskId: number,
  refreshKey: unknown,
  enabled = true,
): TaskResultsState & { refreshFailed: boolean; retry: () => void } {
  const query = useReadQuery({
    queryKey: queryKeys.taskResults(projectId, taskId),
    queryFn: () =>
      getJson<{ results: TaskResult[] }>(`${projectPath(projectId)}/tasks/${taskId}/results`),
    enabled,
  });
  const { refetch } = query;
  // A read that is running is joined, so the read of the first opening is not made twice.
  // biome-ignore lint/correctness/useExhaustiveDependencies: `refreshKey` only makes the read run again.
  useEffect(() => {
    if (enabled) void refetch({ cancelRefetch: false });
  }, [enabled, refreshKey, refetch]);
  const retry = useCallback(() => void refetch({ cancelRefetch: false }), [refetch]);
  const value: TaskResultsState = query.data
    ? { status: 'ready', results: query.data.results }
    : query.isError
      ? { status: 'error' }
      : { status: 'loading' };
  return { ...value, refreshFailed: query.isError && query.data !== undefined, retry };
}
