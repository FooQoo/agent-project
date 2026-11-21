import { useCallback } from 'react';
import { fetchProjects } from '../api';
import type { Project } from '../../../shared/api/contracts';
import { queryKeys } from '../../../shared/data/core/query-keys';
import { useReadQuery } from '../../../shared/data/react';

export interface UseProjectsResult {
  /** `loading` and `error` last until the first list arrives; a later failure keeps the old list. */
  status: 'loading' | 'ready' | 'error';
  projects: Project[];
  catalogVersion: string | null;
  /** True when the last fetch failed. */
  fetchFailed: boolean;
  reload: () => void;
}

const NONE: Project[] = [];

/** The project list, read by its query. `reload` reads it again; a running read is joined. */
export function useProjects(): UseProjectsResult {
  const query = useReadQuery({
    queryKey: queryKeys.projects(),
    queryFn: fetchProjects,
  });
  const { refetch } = query;
  const reload = useCallback(() => void refetch({ cancelRefetch: false }), [refetch]);
  return {
    status: query.data ? 'ready' : query.isError ? 'error' : 'loading',
    projects: query.data?.projects ?? NONE,
    catalogVersion: query.data?.catalogVersion ?? null,
    fetchFailed: query.isError,
    reload,
  };
}
