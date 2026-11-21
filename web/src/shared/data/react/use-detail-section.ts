import { hashKey, useQuery } from '@tanstack/react-query';
import { useMemo, useSyncExternalStore } from 'react';
import type { DetailSection } from '../api/contracts';
import { type DetailEntry, sectionSnapshot } from '../core/detail-snapshot';
import type { ResourceKey, SectionDataMap, SectionSnapshot } from '../core/types';
import { useDataCore } from './provider';

const NONE: ResourceKey = { kind: 'task', projectId: '', id: 0 };

/**
 * One section of a Detail, read by its query. `enabled` is false when no Detail is shown or the
 * section is not wanted; the section then reads as absent.
 */
export function useDetailSection<S extends DetailSection>(
  key: ResourceKey | null,
  section: S,
  enabled: boolean,
): SectionSnapshot<S> {
  const core = useDataCore();
  const options = core.details.options(key ?? NONE, section);
  const query = useQuery<DetailEntry<S>, Error>({ ...options, enabled: key !== null && enabled });
  const hash = hashKey(options.queryKey);
  const partial = useSyncExternalStore(core.details.partials.subscribe, () =>
    core.details.partials.get<S>(hash),
  ) as SectionDataMap[S] | null;
  const { data, isFetching, isError, isStale, error } = query;
  return useMemo(
    () =>
      sectionSnapshot<S>({
        entry: data,
        fetching: isFetching,
        failed: isError,
        stale: isStale,
        error,
        partial,
      }),
    [data, isFetching, isError, isStale, error, partial],
  );
}
