import { useCallback, useMemo, useState } from 'react';
import type { ProjectId } from '@/shared/api/contracts';
import { queryKeys } from '@/shared/data/core/query-keys';
import { useReadQuery } from '@/shared/data/react';
import { useWrite } from '@/shared/data/react/use-write';
import type { ChangedEvent } from '@/shared/data/api/contracts';
import type { PeriodicSummary, SummariesApi } from './model/messages';

function changesSummaries(event: ChangedEvent): boolean {
  return event.changes.some((c) => c.resource === 'messages' && c.facets.includes('summary'));
}

/** The most summaries the tab lists; the server keeps to the same number. */
export const MAX_SUMMARIES = 10;

export interface SummariesController {
  /** The summaries not dismissed, newest first; empty until the first read arrives. */
  summaries: readonly PeriodicSummary[];
  /** The project the summaries belong to; a result link names it. */
  projectId: ProjectId;
  /** The first read has not arrived yet. */
  loading: boolean;
  /** The last dismissal failed: its summary is back in the list. Cleared by the next one. */
  dismissFailed: boolean;
  /** Hides the summary at once and asks the server; a failure brings it back. */
  dismiss: (summaryId: string) => void;
}

/**
 * The periodic summaries of a project. They are read when the list opens and again whenever the
 * stream says the project's `messages` changed with the `summary` facet (or the stream can no
 * longer be trusted). A dismissed summary is hidden before the server answers.
 */
export function useSummaries(
  api: SummariesApi | undefined,
  projectId: ProjectId,
): SummariesController {
  const [hidden, setHidden] = useState<ReadonlySet<string>>(new Set());
  const [dismissFailed, setDismissFailed] = useState(false);
  // A failed read leaves the list as it was (empty at first); the next change or opening reads it again.
  const query = useReadQuery({
    queryKey: queryKeys.messageSummaries(projectId),
    queryFn: () => (api as SummariesApi).list(projectId),
    enabled: api !== undefined,
    changedBy: (event) => event.projectId === projectId && changesSummaries(event),
  });
  const read = query.data ?? (query.isError ? [] : null);

  // Hidden before the server answers; a refusal brings the summary back.
  const dismissWrite = useWrite<unknown, string>({
    mutationKey: ['summary-dismiss', projectId],
    mutationFn: (summaryId) => (api as SummariesApi).dismiss(projectId, summaryId),
    onMutate: (summaryId) => {
      setDismissFailed(false);
      setHidden((current) => new Set(current).add(summaryId));
    },
    onError: (_error, summaryId) => {
      setHidden((current) => {
        const next = new Set(current);
        next.delete(summaryId);
        return next;
      });
      setDismissFailed(true);
    },
  });
  const { mutate } = dismissWrite;
  const dismiss = useCallback(
    (summaryId: string) => {
      if (api) mutate(summaryId);
    },
    [api, mutate],
  );

  const summaries = useMemo(
    () => (read ?? []).filter((summary) => !hidden.has(summary.id)).slice(0, MAX_SUMMARIES),
    [read, hidden],
  );
  return {
    projectId,
    summaries,
    loading: api !== undefined && read === null,
    dismissFailed,
    dismiss,
  };
}
