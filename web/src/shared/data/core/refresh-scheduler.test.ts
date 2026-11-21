// @vitest-environment node
import { type QueryClient, QueryObserver } from '@tanstack/react-query';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ChangeInvalidator } from './change-invalidator';
import { DetailReader } from './detail-reader';
import { PresentationGate } from './presentation-gate';
import { createQueryClient } from './query-client';
import { queryKeys } from './query-keys';
import { RefreshScheduler } from './refresh-scheduler';
import { SummaryReader } from './summary-reader';
import {
  changed,
  FakeAdapter,
  P,
  sectionOf,
  seedDetail,
  showDetail,
  summaryOf,
  task,
} from './test-support';

let adapter: FakeAdapter;
let gate: PresentationGate;
let scheduler: RefreshScheduler;
let client: QueryClient;
let seq = 0;

beforeEach(() => {
  vi.useFakeTimers();
  seq = 0;
  adapter = new FakeAdapter();
  gate = new PresentationGate();
  client = createQueryClient({ retry: false });
  const watched = new Set([P]);
  const summary: SummaryReader = new SummaryReader({
    client,
    adapter,
    versionOf: (key, section) => details.versionOf(key, section),
    gate,
    onCommit: (projectId, data, immediate) => details.reconcile(projectId, data, immediate),
  });
  const details: DetailReader = new DetailReader({
    client,
    adapter,
    gate,
    loadSummary: (projectId) => summary.load(projectId, true),
  });
  const invalidator = new ChangeInvalidator({
    isActiveProject: (id) => watched.has(id),
    hasDetail: (key, section) => details.has(key, section),
  });
  scheduler = new RefreshScheduler({
    gate,
    invalidator,
    summary,
    details,
    watchedProjects: () => watched,
  });
});
afterEach(() => {
  scheduler.dispose();
  client.clear();
  vi.useRealTimers();
});

/** A screen that shows the Summary: the query is active, so a notification refetches it. */
const watch = (seeded: Parameters<typeof summaryOf>[0] | false = []) => {
  if (seeded) client.setQueryData(queryKeys.summary(P), summaryOf(seeded, 'sv0'));
  const observer = new QueryObserver(client, scheduler.summary.options(P));
  return observer.subscribe(() => {});
};
const settle = (ms = 0) => vi.advanceTimersByTimeAsync(ms);
/** A task whose Detail is whole and on screen. */
const seed = (id: number) => {
  seedDetail(client, task(id), 'document', 'd1');
  seedDetail(client, task(id), 'history', 'h1', 'a');
  showDetail(client, scheduler.details, task(id));
};
const note = (
  id: number,
  ...facets: Array<'summary' | 'request' | 'document' | 'history' | 'deleted'>
) => scheduler.onChanged(changed(++seq, [{ resource: 'task', id, facets }]));
const streams = () => adapter.streamCalls.map((c) => c.request.sections.join());
const one = [{ id: 1, document: 'd1', history: 'h1' }];

describe('single-flight', () => {
  it('joins concurrent requests for the Summary of one project', async () => {
    const a = scheduler.loadSummary(P);
    const b = scheduler.loadSummary(P);
    expect(adapter.summaryCalls).toHaveLength(1);
    adapter.summaryCalls[0]?.resolve(summaryOf([]));
    await Promise.all([a, b]);
    expect(scheduler.getSummary(P).state).toBe('ready');
    expect(adapter.summaryCalls).toHaveLength(1);
  });
});

describe('notifications', () => {
  it('asks for the Summary, and not for the Detail, when a section of an open Detail changed', async () => {
    seed(1);
    watch(one);
    note(1, 'document');
    await settle(150);
    expect(adapter.summaryCalls).toHaveLength(1);
    expect(adapter.streamCalls).toHaveLength(0);
  });

  it('asks for nothing for a Detail that is not cached', async () => {
    watch();
    note(99, 'document', 'history');
    await settle(5_000);
    expect(adapter.summaryCalls).toHaveLength(0);
    expect(adapter.streamCalls).toHaveLength(0);
  });

  it('merges notifications of one window into one read of the Summary', async () => {
    seed(1);
    watch(one);
    note(1, 'document');
    note(1, 'history');
    note(1, 'summary');
    await settle(150);
    expect(adapter.summaryCalls).toHaveLength(1);
  });
});

describe('a notification during a read of the Summary', () => {
  it('fetches the Summary again when a notification says more than the response does', async () => {
    seed(1);
    client.setQueryData(
      queryKeys.summary(P),
      summaryOf([{ id: 1, document: 'd1', history: 'h1' }], 'sv1'),
    );
    const done = scheduler.loadSummary(P);
    note(1, 'document');
    adapter.summaryCalls[0]?.resolve(summaryOf([{ id: 1, document: 'd1', history: 'h1' }], 'sv1'));
    await settle(0);
    expect(adapter.summaryCalls).toHaveLength(2);
    expect(scheduler.getSummary(P).state).toBe('refreshing');
    adapter.summaryCalls[1]?.resolve(summaryOf([{ id: 1, document: 'd2', history: 'h1' }], 'sv2'));
    await done;
    await settle(500);
    expect(streams()).toEqual(['document']);
    expect(scheduler.getSummary(P).version).toBe('sv2');
  });

  it('accepts a response that already shows the change', async () => {
    seed(1);
    client.setQueryData(
      queryKeys.summary(P),
      summaryOf([{ id: 1, document: 'd1', history: 'h1' }], 'sv1'),
    );
    const done = scheduler.loadSummary(P);
    note(1, 'document');
    adapter.summaryCalls[0]?.resolve(summaryOf([{ id: 1, document: 'd2', history: 'h1' }], 'sv2'));
    await done;
    await settle(500);
    expect(adapter.summaryCalls).toHaveLength(1);
    expect(streams()).toEqual(['document']);
  });

  it('fetches the first Summary again when a change arrived while it was on the way', async () => {
    const done = scheduler.loadSummary(P);
    note(1, 'summary');
    // The server answered before it read the change, so the answer cannot show it.
    adapter.summaryCalls[0]?.resolve(summaryOf([], 'sv1'));
    await settle(0);
    expect(adapter.summaryCalls).toHaveLength(2);
    adapter.summaryCalls[1]?.resolve(summaryOf([{ id: 1, document: 'd1', history: 'h1' }], 'sv2'));
    await done;
    expect(scheduler.getSummary(P)).toMatchObject({ state: 'ready', version: 'sv2' });
  });

  it('fetches again when a deleted resource is still in the response', async () => {
    seed(1);
    const done = scheduler.loadSummary(P);
    note(1, 'deleted');
    adapter.summaryCalls[0]?.resolve(summaryOf([{ id: 1, document: 'd1', history: 'h1' }]));
    await settle(0);
    expect(adapter.summaryCalls).toHaveLength(2);
    adapter.summaryCalls[1]?.resolve(summaryOf([]));
    await done;
  });

  it('stops asking again after a few contradictions and keeps the last response', async () => {
    client.setQueryData(queryKeys.summary(P), summaryOf([], 'sv1'));
    const done = scheduler.loadSummary(P);
    for (let i = 0; i < 5; i++) {
      scheduler.onChanged(changed(++seq, [{ resource: 'project', id: P, facets: ['summary'] }]));
      adapter.summaryCalls[i]?.resolve(summaryOf([], 'sv1'));
      await settle(0);
    }
    await done;
    expect(adapter.summaryCalls.length).toBeLessThanOrEqual(3);
    expect(scheduler.getSummary(P).state).toBe('ready');
  });
});

describe('cannot be trusted', () => {
  it('after a missing sequence number marks everything stale and settles it with one Summary', async () => {
    seed(1);
    seed(2);
    watch([
      { id: 1, document: 'd1', history: 'h1' },
      { id: 2, document: 'd1', history: 'h1' },
    ]);
    scheduler.onChanged(changed(1, [], 'other'));
    scheduler.onChanged(changed(3, [], 'other'));
    expect(sectionOf(client, task(1), 'document').invalidated).toBe(true);
    expect(scheduler.getSummary(P).state).toBe('stale');
    await settle(150);
    expect(adapter.summaryCalls).toHaveLength(1);
    adapter.summaryCalls[0]?.resolve(
      summaryOf(
        [
          { id: 1, document: 'd1', history: 'h1' },
          { id: 2, document: 'd5', history: 'h1' },
        ],
        'sv2',
      ),
    );
    await settle(150);
    expect(
      adapter.streamCalls.map((c) => `${c.request.resource.id}:${c.request.sections}`),
    ).toEqual(['2:document']);
    expect(sectionOf(client, task(1), 'document').invalidated).toBe(false);
  });

  it('asks for the Summary again after a reconnect', async () => {
    seed(1);
    watch(false);
    scheduler.connected(false);
    expect(adapter.summaryCalls).toHaveLength(1);
    adapter.summaryCalls[0]?.resolve(summaryOf(one));
    await settle(0);
    scheduler.connected(true);
    await settle(150);
    expect(sectionOf(client, task(1), 'document').invalidated).toBe(true);
    expect(adapter.summaryCalls).toHaveLength(2);
  });
});

describe('the presentation gate', () => {
  it('reads nothing while held, and reads the Summary 3 seconds after the release', async () => {
    seed(1);
    watch(one);
    const release = gate.acquire('drawer');
    note(1, 'document');
    await settle(10_000);
    expect(adapter.summaryCalls).toHaveLength(0);
    release();
    await settle(2_999);
    expect(adapter.summaryCalls).toHaveLength(0);
    await settle(1 + 150);
    expect(adapter.summaryCalls).toHaveLength(1);
  });

  it('drops the wait when something is held again', async () => {
    seed(1);
    watch(one);
    const first = gate.acquire('drawer');
    note(1, 'document');
    first();
    await settle(2_000);
    const second = gate.acquire('modal');
    await settle(10_000);
    expect(adapter.summaryCalls).toHaveLength(0);
    second();
    await settle(3_000 + 150);
    expect(adapter.summaryCalls).toHaveLength(1);
  });

  it('does not hold back what the person asked for: a manual refresh reaches the Details too', async () => {
    seed(7);
    gate.acquire('drawer');
    const refresh = scheduler.loadSummary(P, { immediate: true });
    expect(adapter.summaryCalls).toHaveLength(1);
    adapter.summaryCalls[0]?.resolve(summaryOf([{ id: 7, document: 'd5', history: 'h1' }]));
    await refresh;
    await settle(0);
    expect(streams()).toEqual(['document']);
  });

  it('shows a decision being asked or answered at once, with the Details the Summary names', async () => {
    seed(1);
    seed(2);
    watch([
      { id: 1, document: 'd1', history: 'h1' },
      { id: 2, document: 'd1', history: 'h1' },
    ]);
    gate.acquire('drawer');
    note(2, 'document');
    note(1, 'summary', 'request', 'history');
    expect(adapter.summaryCalls).toHaveLength(1);
    adapter.summaryCalls[0]?.resolve(
      summaryOf([
        { id: 1, document: 'd1', history: 'h2' },
        { id: 2, document: 'd2', history: 'h1' },
      ]),
    );
    await settle(0);
    // The Summary is one snapshot: the Details it names other versions of are read with it.
    expect(
      adapter.streamCalls.map((c) => `${c.request.resource.id}:${c.request.sections}`),
    ).toEqual(['1:history', '2:document']);
  });

  it('keeps the connection notifications coming: the sequence is checked while held', async () => {
    seed(1);
    gate.acquire('drawer');
    scheduler.onChanged(changed(1, [], 'other'));
    scheduler.onChanged(changed(5, [], 'other'));
    await settle(10_000);
    expect(adapter.summaryCalls).toHaveLength(0);
  });
});

describe('teardown', () => {
  it('aborts every read on dispose', async () => {
    void scheduler.loadSummary(P);
    showDetail(client, scheduler.details, task(1), ['document']);
    await settle(0);
    scheduler.dispose();
    expect(adapter.summaryCalls[0]?.signal.aborted).toBe(true);
    expect(adapter.streamCalls[0]?.signal.aborted).toBe(true);
    await settle(1_000);
    expect(adapter.summaryCalls).toHaveLength(1);
  });
});
