import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useSyncExternalStore } from 'react';
import type { ProjectId } from '../../api/contracts';
import type { Summary } from '../api/contracts';
import { summaryState } from '../core/summary-reader';
import type { SectionState } from '../core/types';
import { useDataCore } from './provider';

export interface UseSummaryResult {
  state: SectionState;
  summary: Summary | null;
  error: Error | null;
  /** Asks the server again now; the presentation hold does not delay it. */
  refresh(): Promise<void>;
}

/** The Summary of a project, kept current by the change notifications while this is mounted. */
export function useSummary(projectId: ProjectId | null): UseSummaryResult {
  const core = useDataCore();
  useEffect(
    () => (projectId === null ? undefined : core.watchProject(projectId)),
    [core, projectId],
  );
  const canRead = useSyncExternalStore(core.subscribeState, () => core.canReadSummary);
  // Once there is a Summary the query stays on, so the stream reopening never reads it past the gate.
  const loaded = core.getSummary(projectId ?? '').data !== null;
  const query = useQuery<Summary, Error>({
    ...core.summary.options(projectId ?? ''),
    enabled: projectId !== null && (canRead || loaded),
  });
  const refresh = useCallback(
    () => (projectId === null ? Promise.resolve() : core.refreshSummary(projectId)),
    [core, projectId],
  );
  const state = summaryState({
    hasData: query.data !== undefined,
    fetching: query.isFetching,
    failed: query.isError,
    stale: query.isStale,
  });
  return { state, summary: query.data ?? null, error: query.error, refresh };
}
