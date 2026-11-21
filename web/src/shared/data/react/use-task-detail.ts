import { useCallback, useEffect, useMemo } from 'react';
import type { ProjectId } from '../../api/contracts';
import type { DetailSection } from '../api/contracts';
import type { ResourceKey, SectionSnapshot } from '../core/types';
import { useDataCore } from './provider';
import { useDetailSection } from './use-detail-section';

const ALL: DetailSection[] = ['document', 'history'];

export interface UseTaskDetailResult {
  document: SectionSnapshot<'document'>;
  history: SectionSnapshot<'history'>;
  /** Fetches the sections again now; the presentation hold does not delay it. */
  refresh(): Promise<void>;
}

/**
 * A task's Detail. Only `sections` are read, each by its own query; a change of the task that
 * the Summary shows reads the open ones again.
 */
export function useTaskDetail(
  projectId: ProjectId,
  id: number | null,
  sections: DetailSection[] = ALL,
): UseTaskDetailResult {
  const core = useDataCore();
  const key = useMemo<ResourceKey | null>(
    () => (id === null ? null : { kind: 'task', projectId, id }),
    [projectId, id],
  );
  const wanted = sections.join(',');
  const document = useDetailSection(key, 'document', sections.includes('document'));
  const history = useDetailSection(key, 'history', sections.includes('history'));
  // biome-ignore lint/correctness/useExhaustiveDependencies: `wanted` stands for `sections`
  useEffect(() => {
    if (key) core.details.open(key, sections);
  }, [core, key, wanted]);
  // biome-ignore lint/correctness/useExhaustiveDependencies: `wanted` stands for `sections`
  const refresh = useCallback(
    () => (key ? core.details.fetch(key, sections) : Promise.resolve()),
    [core, key, wanted],
  );
  return { document, history, refresh };
}
