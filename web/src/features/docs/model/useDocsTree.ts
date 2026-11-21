import { useCallback } from 'react';
import type { ProjectId } from '@/shared/api/contracts';
import { queryKeys } from '@/shared/data/core/query-keys';
import { useReadClient, useReadQuery } from '@/shared/data/react';
import { type DocsTree, fetchDocsTree } from '../api/docsApi';

export type DocsTreeState =
  | { status: 'loading'; tree: null }
  | { status: 'error'; tree: null }
  | { status: 'ready'; tree: DocsTree };

/**
 * The file list of a project's docs. It is read when the dialog opens (this hook mounts) and
 * again when `reload` is called; opening or closing a folder does not read. `refresh` reads it
 * again without the loading state, for a change made outside: the list stays until the new one
 * arrives, and a failed read keeps it.
 */
export function useDocsTree(projectId: ProjectId) {
  const client = useReadClient();
  const query = useReadQuery(
    {
      queryKey: queryKeys.docsTree(projectId),
      queryFn: () => fetchDocsTree(projectId),
    },
    client,
  );
  const { refetch } = query;
  const reload = useCallback(() => void refetch({ cancelRefetch: false }), [refetch]);
  const refresh = useCallback(async (): Promise<DocsTree | null> => {
    try {
      return await client.fetchQuery({
        queryKey: queryKeys.docsTree(projectId),
        queryFn: () => fetchDocsTree(projectId),
        staleTime: 0,
        retry: false,
      });
    } catch {
      return null;
    }
  }, [client, projectId]);
  return {
    status: query.data ? 'ready' : query.isError ? 'error' : 'loading',
    nodes: query.data?.nodes ?? [],
    truncated: query.data?.truncated ?? false,
    reload,
    refresh,
  } as const;
}
