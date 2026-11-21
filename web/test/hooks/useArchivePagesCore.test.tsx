import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { FetchError } from '../../src/shared/api/http';
import type { ChangedEvent } from '../../src/shared/data/api/contracts';
import { fakeCore } from '../../src/shared/data/react/test-support';
import { PresentationGate } from '../../src/shared/data/core/presentation-gate';
import {
  type ArchivePage,
  type ArchivedTask,
  fetchArchivePage,
} from '../../src/features/tasks/api/read';
import { useArchivePages } from '../../src/features/tasks/model/useArchivePages';

vi.mock('../../src/features/tasks/api/read', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../src/features/tasks/api/read')>()),
  fetchArchivePage: vi.fn(),
}));
const fetchPage = vi.mocked(fetchArchivePage);

const P1 = '0f1e2d3c4b5a69788796a5b4c3d2e1f0';
const P2 = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

const row = (id: number): ArchivedTask => ({
  id,
  title: `Archived ${id}`,
  assignee: 'ai',
  taskStatus: 'done',
  taskStatusChangedAt: '2026-10-01T09:00:00.000Z',
  position: 1,
  agentStatus: 'idle',
  pendingReason: null,
  pendingRequestId: null,
  pendingChoices: [],
  epic: null,
  createdAt: '2026-10-01T09:00:00.000Z',
  updatedAt: '2026-10-01T09:00:00.000Z',
  agentStatusChangedAt: '2026-10-01T09:00:00.000Z',
  draft: null,
  acceptance: { done: 0, total: 0 },
  // A higher id was archived later, so it is listed first.
  archivedAt: `2026-10-02T09:00:${String(id).padStart(2, '0')}.000Z`,
});

const page = (
  projectId: string,
  ids: number[],
  nextCursor: string | null,
  total = 5,
): ArchivePage => ({
  project: { id: projectId as never, name: projectId, createdAt: '2026-10-01T08:00:00.000Z' },
  version: 'v',
  tasks: ids.map(row),
  epics: [],
  total,
  nextCursor,
});

const changed = (projectId: string): ChangedEvent => ({
  instanceId: 'i',
  seq: 1,
  projectId,
  changes: [{ resource: 'task', id: 1, facets: ['summary'] }],
});

const ids = (hook: { current: ReturnType<typeof useArchivePages> }) =>
  hook.current.data?.tasks.map((task) => task.id);

function setup(initial: string | null = P1) {
  const fake = fakeCore();
  (fake.core as unknown as { gate: PresentationGate }).gate = new PresentationGate();
  const hook = renderHook(({ id }) => useArchivePages(id as never), {
    wrapper: fake.wrapper,
    initialProps: { id: initial },
  });
  return { ...fake, ...hook };
}

beforeEach(() => fetchPage.mockReset());
afterEach(cleanup);

describe('useArchivePages with a data core', () => {
  it('reads the first page, then the next on request, and ends after the last', async () => {
    fetchPage
      .mockResolvedValueOnce(page(P1, [5, 4], 'c1'))
      .mockResolvedValueOnce(page(P1, [3], null));
    const { result } = setup();
    expect(result.current.status).toBe('loading');
    await waitFor(() => expect(result.current.status).toBe('ready'));
    expect(ids(result)).toEqual([5, 4]);
    expect(result.current.hasMore).toBe(true);
    expect(fetchPage).toHaveBeenLastCalledWith(P1, null);

    act(() => result.current.loadMore());
    await waitFor(() => expect(ids(result)).toEqual([5, 4, 3]));
    expect(fetchPage).toHaveBeenLastCalledWith(P1, 'c1');
    expect(ids(result)).toEqual([5, 4, 3]);
    expect(result.current.hasMore).toBe(false);
  });

  it('shows Retry for a failed page without a hidden retry, and Retry recovers', async () => {
    fetchPage
      .mockResolvedValueOnce(page(P1, [5, 4], 'c1'))
      .mockRejectedValueOnce(new Error('boom'))
      .mockResolvedValueOnce(page(P1, [3], null));
    const { result } = setup();
    await waitFor(() => expect(result.current.status).toBe('ready'));
    act(() => result.current.loadMore());
    await waitFor(() => expect(result.current.loadMoreFailed).toBe(true));
    // The rows already read stay, the failure is the next page's only, and nothing asked again.
    expect(ids(result)).toEqual([5, 4]);
    expect(result.current.fetchFailed).toBe(false);
    expect(result.current.status).toBe('ready');
    expect(fetchPage).toHaveBeenCalledTimes(2);

    act(() => result.current.retryMore());
    await waitFor(() => expect(ids(result)).toEqual([5, 4, 3]));
    expect(result.current.loadMoreFailed).toBe(false);
  });

  it('shows an error for a failed first read, and reload recovers', async () => {
    fetchPage.mockRejectedValueOnce(new Error('boom')).mockResolvedValueOnce(page(P1, [5], null));
    const { result } = setup();
    await waitFor(() => expect(result.current.status).toBe('error'));
    expect(result.current.error).toBe('failed');
    expect(result.current.fetchFailed).toBe(true);
    expect(fetchPage).toHaveBeenCalledTimes(1);
    act(() => result.current.reload());
    await waitFor(() => expect(result.current.status).toBe('ready'));
    expect(ids(result)).toEqual([5]);
    expect(result.current.fetchFailed).toBe(false);
  });

  it('reports a project that is gone as not found', async () => {
    fetchPage.mockRejectedValueOnce(new FetchError('gone', 404, 'project_not_found'));
    const { result } = setup();
    await waitFor(() => expect(result.current.status).toBe('error'));
    expect(result.current.error).toBe('not_found');
  });

  it('keeps the rows shown when a refresh fails', async () => {
    fetchPage.mockResolvedValueOnce(page(P1, [5], null)).mockRejectedValueOnce(new Error('boom'));
    const { result, emit } = setup();
    await waitFor(() => expect(result.current.status).toBe('ready'));
    await emit(changed(P1));
    await waitFor(() => expect(result.current.fetchFailed).toBe(true));
    expect(result.current.status).toBe('ready');
    expect(ids(result)).toEqual([5]);
  });

  it('shows no duplicate and no gap when a task is archived or restored between pages', async () => {
    // Read: 5 4 | (task 6 archived, 4 restored) | 3 2. The keyset cursor names the last row read.
    fetchPage
      .mockResolvedValueOnce(page(P1, [5, 4], 'after-4'))
      .mockResolvedValueOnce(page(P1, [4, 3], 'after-3'));
    const { result } = setup();
    await waitFor(() => expect(result.current.status).toBe('ready'));
    act(() => result.current.loadMore());
    await waitFor(() => expect(ids(result)).toEqual([5, 4, 3]));
    expect(new Set(ids(result)).size).toBe(3);

    // The window is read again from the newest page on and swapped in at once.
    fetchPage
      .mockResolvedValueOnce(page(P1, [6, 5], 'after-5', 5))
      .mockResolvedValueOnce(page(P1, [3, 2], null, 5));
    await act(async () => result.current.reload());
    await waitFor(() => expect(ids(result)).toEqual([6, 5, 3, 2]));
    expect(result.current.hasMore).toBe(false);
    expect(fetchPage).toHaveBeenLastCalledWith(P1, 'after-5');
  });

  it('reads again for a change of its project or a reset, not for another project', async () => {
    fetchPage.mockImplementation(async (id) => page(id, [5], null, 1));
    const { result, emit } = setup();
    await waitFor(() => expect(result.current.status).toBe('ready'));
    await emit(changed(P2));
    expect(fetchPage).toHaveBeenCalledTimes(1);
    await emit(changed(P1));
    await waitFor(() => expect(fetchPage).toHaveBeenCalledTimes(2));
    await emit(null);
    await waitFor(() => expect(fetchPage).toHaveBeenCalledTimes(3));
  });
});
