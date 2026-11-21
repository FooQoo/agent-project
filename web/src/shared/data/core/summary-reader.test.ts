// @vitest-environment node
import { QueryObserver } from '@tanstack/react-query';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DataCore } from './data-core';
import type { VisibilitySource } from './event-connection';
import { createQueryClient, SUMMARY_GC_TIME_MS } from './query-client';
import { queryKeys } from './query-keys';
import { summaryState } from './summary-reader';
import { changed, FakeAdapter, P, summaryOf } from './test-support';

let adapter: FakeAdapter;
let core: DataCore;
let seq = 0;
const visible: VisibilitySource = { hidden: () => false, subscribe: () => () => {} };
const settle = (ms = 0) => vi.advanceTimersByTimeAsync(ms);

/** What `useSummary` does: watches the project and reads the Summary once the core allows it. */
function show() {
  core.watchProject(P);
  const options = () => ({ ...core.summary.options(P), enabled: core.canReadSummary });
  const observer = new QueryObserver(core.queryClient, options());
  core.subscribeState(() => observer.setOptions(options()));
  observer.subscribe(() => {});
}
const touch = () =>
  adapter.handlers?.onChanged(
    changed(++seq, [{ resource: 'project', id: P, facets: ['summary'] }]),
  );
const answer = async (call: number, version: string) => {
  adapter.summaryCalls[call]?.resolve(summaryOf([], version));
  await settle(0);
};

beforeEach(() => {
  vi.useFakeTimers();
  seq = 0;
  adapter = new FakeAdapter();
  core = new DataCore({ adapter, visibility: visible });
  core.start();
  adapter.handlers?.onOpen?.();
  show();
});
afterEach(() => {
  core.dispose();
  vi.useRealTimers();
});

describe('the Summary as a query', () => {
  it('is read once for the project and held under its key', async () => {
    expect(adapter.summaryCalls).toHaveLength(1);
    await answer(0, 'sv1');
    expect(core.queryClient.getQueryData(queryKeys.summary(P))).toMatchObject({ version: 'sv1' });
    expect(core.getSummary(P)).toMatchObject({ state: 'ready', version: 'sv1' });
    await settle(60_000);
    expect(adapter.summaryCalls).toHaveLength(1); // nothing is read again by age
  });

  it('is read again after a change notification', async () => {
    await answer(0, 'sv1');
    touch();
    await settle(150);
    expect(adapter.summaryCalls).toHaveLength(2);
    await answer(1, 'sv2');
    expect(core.getSummary(P)).toMatchObject({ state: 'ready', version: 'sv2' });
  });

  it('keeps the Summary a long time after nobody shows it', () => {
    expect(core.summary.options(P).gcTime).toBe(SUMMARY_GC_TIME_MS);
  });
});

describe('the presentation gate holds the instruction, not the fetch', () => {
  beforeEach(() => answer(0, 'sv1'));

  it('refetches nothing while closed, and once after it opens', async () => {
    const release = core.gate.acquire('drawer');
    touch();
    touch();
    await settle(20_000);
    expect(adapter.summaryCalls).toHaveLength(1);
    release();
    await settle(2_999);
    expect(adapter.summaryCalls).toHaveLength(1);
    await settle(1 + 150);
    expect(adapter.summaryCalls).toHaveLength(2);
    await answer(1, 'sv2');
    await settle(10_000);
    expect(adapter.summaryCalls).toHaveLength(2);
    expect(core.getSummary(P).version).toBe('sv2');
  });

  it('cancels a fetch in flight when it closes, keeps what was shown, and refetches after it opens', async () => {
    touch();
    await settle(150);
    expect(adapter.summaryCalls).toHaveLength(2);
    const release = core.gate.acquire('modal');
    await settle(0);
    expect(adapter.summaryCalls[1]?.signal.aborted).toBe(true);
    expect(core.getSummary(P)).toMatchObject({ state: 'stale', version: 'sv1' });
    await settle(20_000);
    expect(adapter.summaryCalls).toHaveLength(2);
    release();
    await settle(3_000 + 150);
    expect(adapter.summaryCalls).toHaveLength(3);
    await answer(2, 'sv3');
    expect(core.getSummary(P)).toMatchObject({ state: 'ready', version: 'sv3' });
  });

  it('lets a manual refresh and a decision request through', async () => {
    core.gate.acquire('modal');
    void core.refreshSummary(P);
    expect(adapter.summaryCalls).toHaveLength(2);
    await answer(1, 'sv2');
    adapter.handlers?.onChanged(
      changed(++seq, [{ resource: 'task', id: 1, facets: ['summary', 'request'] }]),
    );
    await settle(0);
    expect(adapter.summaryCalls).toHaveLength(3);
  });

  it('does not cancel a fetch a person asked for', async () => {
    void core.refreshSummary(P);
    core.gate.acquire('modal');
    await settle(0);
    expect(adapter.summaryCalls[1]?.signal.aborted).toBe(false);
  });
});

describe('rapid notifications', () => {
  it('join the read that is running instead of restarting it, and are covered by its check', async () => {
    await answer(0, 'sv1');
    touch();
    await settle(150);
    expect(adapter.summaryCalls).toHaveLength(2);
    touch();
    await settle(150);
    expect(adapter.summaryCalls).toHaveLength(2);
    expect(adapter.summaryCalls[1]?.signal.aborted).toBe(false);
    // The response is no newer than the notification that arrived meanwhile: it is read again.
    await answer(1, 'sv1');
    expect(adapter.summaryCalls).toHaveLength(3);
    await answer(2, 'sv2');
    expect(core.getSummary(P).version).toBe('sv2');
  });
});

describe('a reset during a read', () => {
  it('is settled only by a read that began after it, and that read wins', async () => {
    expect(adapter.summaryCalls).toHaveLength(1); // the first read is still on the way
    core.scheduler.onReset('reconnected');
    await settle(150);
    // The running read began before the reset and may predate missed changes: it is restarted.
    expect(adapter.summaryCalls[0]?.signal.aborted).toBe(true);
    expect(adapter.summaryCalls).toHaveLength(2);
    adapter.summaryCalls[0]?.resolve(summaryOf([], 'old')); // a late answer of the old read
    await answer(1, 'new');
    expect(core.getSummary(P)).toMatchObject({ state: 'ready', version: 'new' });
    await settle(1_000);
    expect(adapter.summaryCalls).toHaveLength(2);
  });
});

describe('a screen that leaves during a read', () => {
  it('finds the Summary stale when it comes back, after the read was cancelled', async () => {
    core.dispose();
    adapter = new FakeAdapter();
    core = new DataCore({ adapter, visibility: visible });
    core.start();
    adapter.handlers?.onOpen?.();
    const leave = core.watchProject(P);
    const first = core.refreshSummary(P);
    await answer(0, 'sv1');
    await first;
    void core.refreshSummary(P);
    leave();
    // The cancelled read restores the state from before it, without the stale mark.
    await core.queryClient.cancelQueries({ queryKey: queryKeys.summary(P) }, { revert: true });
    core.watchProject(P);
    expect(core.getSummary(P)).toMatchObject({ state: 'stale', version: 'sv1' });
  });
});

describe('a Summary read before the first subscription', () => {
  it('is read again when the subscription opens', async () => {
    core.dispose();
    adapter = new FakeAdapter();
    core = new DataCore({ adapter, visibility: { hidden: () => true, subscribe: () => () => {} } });
    core.start();
    show();
    await settle(0);
    expect(adapter.summaryCalls).toHaveLength(1); // hidden tab: read without the stream
    await answer(0, 'sv1');
    core.scheduler.connected(false);
    await settle(150);
    expect(adapter.summaryCalls).toHaveLength(2);
    await answer(1, 'sv2');
    expect(core.getSummary(P).version).toBe('sv2');
  });

  it('is not read again when the first read waited for the subscription', async () => {
    core.dispose();
    adapter = new FakeAdapter();
    core = new DataCore({ adapter, visibility: visible });
    core.start();
    show(); // waits: the stream is still connecting
    await settle(0);
    expect(adapter.summaryCalls).toHaveLength(0);
    adapter.handlers?.onOpen?.();
    await settle(0);
    await answer(0, 'sv1');
    await settle(1_000);
    expect(adapter.summaryCalls).toHaveLength(1);
  });
});

describe('a response against the notifications that arrived while it was fetched', () => {
  it('is read again when it shows no sign of the change', async () => {
    await answer(0, 'sv1');
    touch();
    await settle(150);
    touch(); // arrives while the second read runs
    await answer(1, 'sv1'); // the response is as old as the one before
    expect(adapter.summaryCalls.length).toBeGreaterThanOrEqual(3);
    await answer(2, 'sv2');
    expect(core.getSummary(P).version).toBe('sv2');
  });
});

describe('the query client', () => {
  it('reads again only when asked, and tries a failed read twice more unless the server refused it', () => {
    const queries = createQueryClient({ retry: true }).getDefaultOptions().queries;
    expect(queries).toMatchObject({
      staleTime: Number.POSITIVE_INFINITY,
      refetchOnWindowFocus: false,
      refetchOnReconnect: false,
      networkMode: 'always',
    });
    const retry = queries?.retry as (count: number, error: Error) => boolean;
    expect(retry(0, new Error('down'))).toBe(true);
    expect(retry(1, new Error('down'))).toBe(true);
    expect(retry(2, new Error('down'))).toBe(false);
    expect(retry(0, Object.assign(new Error('gone'), { status: 404 }))).toBe(false);
    expect(retry(0, Object.assign(new Error('broken'), { status: 500 }))).toBe(true);
    expect(createQueryClient().getDefaultOptions().mutations?.retry).toBe(false);
  });
});

describe('a read a person asked for', () => {
  it('fails at once, without the retries of a background read', async () => {
    const retrying = new DataCore({
      adapter,
      visibility: visible,
      queryClient: createQueryClient({ retry: true }),
    });
    const before = adapter.summaryCalls.length;
    const done = retrying.refreshSummary(P);
    adapter.summaryCalls[before]?.reject(new Error('down'));
    await done;
    await settle(10_000);
    expect(adapter.summaryCalls).toHaveLength(before + 1);
    expect(retrying.getSummary(P).state).toBe('error');
    retrying.dispose();
  });
});

describe('summaryState', () => {
  const base = { hasData: true, fetching: false, failed: false, stale: false };
  it('names what a screen shows', () => {
    expect(summaryState({ ...base, hasData: false })).toBe('absent');
    expect(summaryState({ ...base, hasData: false, fetching: true })).toBe('loading');
    expect(summaryState({ ...base, hasData: false, failed: true })).toBe('error');
    expect(summaryState(base)).toBe('ready');
    expect(summaryState({ ...base, fetching: true })).toBe('refreshing');
    expect(summaryState({ ...base, stale: true })).toBe('stale');
    expect(summaryState({ ...base, failed: true })).toBe('stale');
  });
});
