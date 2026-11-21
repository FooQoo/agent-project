import { useCallback, useState } from 'react';
import { FetchError } from '@/shared/api/http';
import type { ProjectId } from '@/shared/api/contracts';
import { queryKeys } from '@/shared/data/core/query-keys';
import { useReadClient, useReadQuery } from '@/shared/data/react';
import { type DocsFile, fetchDocsFile } from '../api/docsApi';

export type DocumentState =
  | { status: 'idle' | 'loading' | 'error'; file: null }
  | { status: 'ready'; file: DocsFile };

/** What a quiet read of the open file found. */
export type FileRefresh = 'updated' | 'unchanged' | 'deleted' | 'failed';

/**
 * One file of the docs, read when `path` is chosen (nothing is read while it is null).
 * `loadId` changes with every fresh load. `refresh` reads the file again without the loading
 * state, for a change made outside: the shown file is replaced only when its revision differs,
 * and is kept when the file is gone or the read fails.
 */
export function useDocument(projectId: ProjectId, path: string | null) {
  const client = useReadClient();
  const [attempt, setAttempt] = useState(0);
  const query = useReadQuery(
    {
      queryKey: queryKeys.docsFile(projectId, path ?? ''),
      queryFn: () => fetchDocsFile(projectId, path as string),
      enabled: path !== null,
    },
    client,
  );
  const { refetch } = query;
  const reload = useCallback(() => {
    setAttempt((value) => value + 1);
    void refetch({ cancelRefetch: false });
  }, [refetch]);
  const refresh = useCallback(async (): Promise<FileRefresh> => {
    if (path === null) return 'unchanged';
    const key = queryKeys.docsFile(projectId, path);
    const shown = client.getQueryData<DocsFile>(key);
    try {
      const file = await client.fetchQuery({
        queryKey: key,
        queryFn: () => fetchDocsFile(projectId, path),
        staleTime: 0,
        retry: false,
      });
      return shown === undefined || shown.revision === file.revision ? 'unchanged' : 'updated';
    } catch (cause) {
      return cause instanceof FetchError && cause.status === 404 ? 'deleted' : 'failed';
    }
  }, [client, projectId, path]);
  const state: DocumentState =
    path === null
      ? { status: 'idle', file: null }
      : query.data
        ? { status: 'ready', file: query.data }
        : { status: query.isError ? 'error' : 'loading', file: null };
  return {
    status: state.status,
    file: state.file,
    loadId: `${path}:${attempt}`,
    reload,
    refresh,
  };
}
