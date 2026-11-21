import { useCallback, useEffect, useMemo } from 'react';
import type { ProjectId } from '../../api/contracts';
import type { ResourceKey, SectionSnapshot } from '../core/types';
import { useDataCore } from './provider';
import { useDetailSection } from './use-detail-section';

export interface UseEpicDetailResult {
  document: SectionSnapshot<'document'>;
  /** Fetches the document again now; the presentation hold does not delay it. */
  refresh(): Promise<void>;
}

/** An epic's document, read by its query. */
export function useEpicDetail(projectId: ProjectId, id: string | null): UseEpicDetailResult {
  const core = useDataCore();
  const key = useMemo<ResourceKey | null>(
    () => (id === null ? null : { kind: 'epic', projectId, id }),
    [projectId, id],
  );
  const document = useDetailSection(key, 'document', true);
  useEffect(() => {
    if (key) core.details.open(key, ['document']);
  }, [core, key]);
  const refresh = useCallback(
    () => (key ? core.details.fetch(key, ['document']) : Promise.resolve()),
    [core, key],
  );
  return { document, refresh };
}
