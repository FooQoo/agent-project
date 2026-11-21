// The archive of a project, read a page at a time through the query client. One infinite query per
// project holds the pages; the query client does the paging state, the joining of reads, and the
// dropping of a read that a project change has made useless.
import {
  type InfiniteData,
  type QueryClient,
  type UseInfiniteQueryResult,
  useInfiniteQuery,
} from '@tanstack/react-query';
import { useCallback, useEffect } from 'react';
import { FetchError } from '../../../shared/api/http';
import type { Epic, ProjectId } from '../../../shared/api/contracts';
import { queryKeys } from '../../../shared/data/core/query-keys';
import { useOptionalDataCore, useReadClient } from '../../../shared/data/react';
import {
  type ArchivePage,
  type ArchiveResponse,
  type ArchivedTask,
  compareArchivedNewestFirst,
  fetchArchivePage,
} from '../api/read';

export interface ArchivePages {
  status: 'loading' | 'ready' | 'error';
  /** The pages read so far, joined; `null` until the first page arrives. */
  data: ArchiveResponse | null;
  error: 'not_found' | 'failed' | null;
  /** A refresh of the pages read failed, but the rows shown are from the last good read. */
  fetchFailed: boolean;
  lastSyncedAt: Date | null;
  /** More pages follow the rows shown. */
  hasMore: boolean;
  loadingMore: boolean;
  /** The last request for the next page failed; `loadMore` or `retryMore` asks again. */
  loadMoreFailed: boolean;
  /** Asks for the next page; ignored while a read is on its way or after the last. */
  loadMore: () => void;
  /** Asks again after a failure of the next page. */
  retryMore: () => void;
  /** Reads the pages loaded again from the newest on (a live update, or Retry). */
  reload: () => void;
  /** Drops a row at once (it was restored here); the archive is not read again. */
  remove: (id: number) => void;
}

type Pages = InfiniteData<ArchivePage, string | null>;
type Joined = ArchiveResponse & { nextCursor: string | null };

/**
 * The pages joined: rows sorted newest first with a row that appears twice kept once (the later copy
 * wins), so a task archived or restored between two page reads never shows twice. The cursor is a
 * keyset (it names the last row read, not an offset), so no row is skipped either.
 */
function join({ pages }: Pages): Joined {
  const tasks = new Map<number, ArchivedTask>();
  const epics = new Map<string, Epic>();
  for (const page of pages) {
    for (const row of page.tasks) tasks.set(row.id, row);
    for (const epic of page.epics) epics.set(epic.id, epic);
  }
  const first = pages[0] as ArchivePage;
  const last = pages[pages.length - 1] as ArchivePage;
  return {
    project: first.project,
    version: first.version,
    tasks: [...tasks.values()].sort(compareArchivedNewestFirst),
    epics: [...epics.values()],
    total: Math.max(...pages.map((page) => page.total)),
    nextCursor: last.nextCursor,
  };
}

/** Takes a row out of the pages read, so a restored task leaves the list before any new read. */
function withoutRow(pages: Pages | undefined, id: number): Pages | undefined {
  if (!pages?.pages.some((page) => page.tasks.some((row) => row.id === id))) return pages;
  return {
    ...pages,
    pages: pages.pages.map((page) => ({
      ...page,
      tasks: page.tasks.filter((row) => row.id !== id),
      total: Math.max(0, page.total - 1),
    })),
  };
}

/** Reads the pages loaded again when the stream says the project changed, or can no longer be trusted. */
function useLiveArchive(client: QueryClient, projectId: ProjectId | null) {
  const core = useOptionalDataCore();
  useEffect(() => {
    if (!core || projectId === null) return undefined;
    const queryKey = queryKeys.archive(projectId);
    return core.subscribeChanges((event) => {
      if (event !== null && event.projectId !== projectId) return;
      // Like the Summary, it waits while a modal is open. A read in progress is cancelled and
      // started again: the pages it would bring were cut at a cursor that is no longer current.
      core.gate.submit(
        () => {
          // With no rows yet TanStack joins a running read instead of restarting it, and its
          // success would clear the invalidation, so a read that began before the change is
          // cancelled outright and the refetch that follows starts from the current state.
          void client
            .cancelQueries({ queryKey, exact: true })
            .then(() => client.invalidateQueries({ queryKey, exact: true }));
        },
        { key: 'archive' },
      );
    });
  }, [core, client, projectId]);
}

function view(
  query: UseInfiniteQueryResult<Joined, Error>,
): Omit<ArchivePages, 'loadMore' | 'retryMore' | 'reload' | 'remove'> {
  const { data, error } = query;
  const notFound = error instanceof FetchError && error.projectNotFound;
  // A project that is gone, or a first read that failed, leaves no rows to show.
  const failed = query.isError && (data === undefined || notFound);
  const refreshFailed = query.isError && data !== undefined && !query.isFetchNextPageError;
  return {
    status: failed ? 'error' : data === undefined ? 'loading' : 'ready',
    data: failed ? null : (data ?? null),
    error: failed ? (notFound ? 'not_found' : 'failed') : null,
    fetchFailed: query.isError && (failed || refreshFailed),
    lastSyncedAt:
      data !== undefined && query.dataUpdatedAt > 0 ? new Date(query.dataUpdatedAt) : null,
    hasMore: query.hasNextPage,
    loadingMore: query.isFetchingNextPage,
    loadMoreFailed: query.isFetchNextPageError,
  };
}

/**
 * The archive of a project, read a page at a time and kept newest first. Used by the PC list and the
 * phone sheet. Each project has its own query, so another project's rows are never shown; leaving
 * the archive (a `null` project) drops what was read, and the next opening reads the first page.
 * A failed read does not try again by itself: the screen shows Retry.
 */
export function useArchivePages(projectId: ProjectId | null): ArchivePages {
  const client = useReadClient();
  const queryKey = queryKeys.archive(projectId ?? '');
  const query = useInfiniteQuery<ArchivePage, Error, Joined, readonly unknown[], string | null>(
    {
      queryKey,
      queryFn: ({ pageParam }) => fetchArchivePage(projectId as ProjectId, pageParam),
      initialPageParam: null,
      getNextPageParam: (last) => last.nextCursor,
      select: join,
      enabled: projectId !== null,
      gcTime: 0,
      retry: false,
    },
    client,
  );
  useLiveArchive(client, projectId);

  const { fetchNextPage, refetch, hasNextPage } = query;
  const loadMore = useCallback(() => {
    // A read in progress (a refresh of the window, or a page) is joined, not asked twice.
    if (hasNextPage) void fetchNextPage({ cancelRefetch: false });
  }, [fetchNextPage, hasNextPage]);
  // The newest state wins: a read already running is cancelled and started again.
  const reload = useCallback(() => {
    if (projectId !== null) void refetch();
  }, [refetch, projectId]);
  const remove = useCallback(
    (id: number) =>
      void client.setQueryData<Pages>(queryKeys.archive(projectId ?? ''), (pages) =>
        withoutRow(pages, id),
      ),
    [client, projectId],
  );
  return { ...view(query), loadMore, retryMore: loadMore, reload, remove };
}
