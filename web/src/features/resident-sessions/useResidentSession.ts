import { useCallback, useEffect } from 'react';
import type { ProjectId } from '../../shared/api/contracts';
import { queryKeys } from '../../shared/data/core/query-keys';
import { useReadQuery } from '../../shared/data/react';
import { getSessions } from './api';
import { onResidentChanged } from './events';

/** The footer asks this often; the open dialog asks faster, because its buttons wait on the answer. */
export const FOOTER_POLL_MS = 5000;
export const DIALOG_POLL_MS = 2000;

/**
 * The sessions of a project, read from `GET /api/projects/{id}/sessions` every `intervalMs` (null
 * stops it), after each change the dialog reports, and when the page becomes visible again. The
 * server sends no event for sessions, so this is the only refresh. A failed read keeps the last
 * snapshot and sets `failed`. The query is keyed by project, so another project never shows the
 * previous one's snapshot.
 */
export function useResidentSession(projectId: ProjectId | null, intervalMs: number | null) {
  const query = useReadQuery({
    queryKey: queryKeys.residentSessions(projectId ?? ''),
    queryFn: () => getSessions(projectId as ProjectId),
    enabled: projectId !== null && intervalMs !== null,
    refetchIntervalMs: intervalMs ?? undefined,
  });
  const { refetch } = query;
  // A read that is running is joined, so a request is never made twice at once.
  const refresh = useCallback(async () => {
    if (projectId !== null) await refetch({ cancelRefetch: false });
  }, [projectId, refetch]);

  useEffect(() => {
    if (projectId === null || intervalMs === null) return;
    const visible = () => {
      if (document.visibilityState === 'visible') void refresh();
    };
    document.addEventListener('visibilitychange', visible);
    const stopChanges = onResidentChanged(() => void refresh());
    return () => {
      document.removeEventListener('visibilitychange', visible);
      stopChanges();
    };
  }, [projectId, intervalMs, refresh]);

  return {
    snapshot: projectId === null ? null : (query.data ?? null),
    failed: projectId !== null && query.isError,
  };
}
