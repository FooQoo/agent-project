// @vitest-environment node
import { QueryObserver } from '@tanstack/react-query';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DataCore } from './data-core';
import type { VisibilitySource } from './event-connection';
import { createQueryClient } from './query-client';
import { queryKeys } from './query-keys';
import {
  changed,
  FakeAdapter,
  P,
  sectionOf,
  seedDetail,
  showDetail,
  summaryOf,
  task,
  taskStream,
} from './test-support';

let adapter: FakeAdapter;
let core: DataCore;
let seq = 0;
const visible: VisibilitySource = { hidden: () => false, subscribe: () => () => {} };
const settle = (ms = 0) => vi.advanceTimersByTimeAsync(ms);
const sv = (version: string, document: string, history = 'h1') =>
  summaryOf([{ id: 1, document, history }], version);
const sectionsAsked = () => adapter.streamCalls.map((c) => c.request.sections.join());
const note = (...facets: Array<'summary' | 'document' | 'history' | 'request'>) =>
  adapter.handlers?.onChanged(changed(++seq, [{ resource: 'task', id: 1, facets }]));

/** The board shows the project, and a task Detail is open with both sections whole. */
function openTask(options: { retry?: boolean } = {}) {
  core = new DataCore({
    adapter,
    visibility: visible,
    queryClient: createQueryClient({ retry: options.retry ?? false }),
  });
  core.start();
  adapter.handlers?.onOpen?.();
  core.queryClient.setQueryData(queryKeys.summary(P), sv('sv1', 'd1'));
  core.watchProject(P);
  new QueryObserver(core.queryClient, core.summary.options(P)).subscribe(() => {});
  seedDetail(core.queryClient, task(1), 'document', 'd1');
  seedDetail(core.queryClient, task(1), 'history', 'h1', 'a');
  return showDetail(core.queryClient, core.details, task(1));
}
/** The Summary the next read of it returns. */
const answerSummary = async (call: number, summary: ReturnType<typeof sv>) => {
  adapter.summaryCalls[call]?.resolve(summary);
  await settle(0);
};

beforeEach(() => {
  vi.useFakeTimers();
  seq = 0;
  adapter = new FakeAdapter();
});
afterEach(() => {
  core.dispose();
  vi.useRealTimers();
});

describe('an open Detail follows a change', () => {
  it('reads again only the section whose version the new Summary changed', async () => {
    openTask();
    note('history');
    await settle(150);
    expect(adapter.summaryCalls).toHaveLength(1);
    await answerSummary(0, sv('sv2', 'd1', 'h2'));
    expect(sectionsAsked()).toEqual(['history']);
    adapter.streamCalls[0]?.respond(taskStream(1, { history: 'h2' }, { reasons: ['a', 'b'] }));
    await settle(0);
    expect(sectionOf(core.queryClient, task(1), 'history')).toMatchObject({
      version: 'h2',
      invalidated: false,
    });
    expect(sectionOf(core.queryClient, task(1), 'document').version).toBe('d1');
    expect(adapter.streamCalls).toHaveLength(1);
  });

  it('holds the stream to the version the Summary named', async () => {
    openTask();
    note('document');
    await settle(150);
    await answerSummary(0, sv('sv2', 'd2'));
    expect(adapter.streamCalls[0]?.request.sections).toEqual(['document']);
    // The server is ahead of the Summary: the stream is not kept, the Summary is read again.
    adapter.streamCalls[0]?.respond(taskStream(1, { document: 'd3' }));
    await settle(0);
    expect(adapter.summaryCalls).toHaveLength(2);
    await answerSummary(1, sv('sv3', 'd3'));
    expect(adapter.streamCalls).toHaveLength(2);
    adapter.streamCalls[1]?.respond(taskStream(1, { document: 'd3' }));
    await settle(0);
    expect(sectionOf(core.queryClient, task(1), 'document').version).toBe('d3');
  });

  it('shows what has arrived while the stream runs', async () => {
    openTask();
    note('document');
    await settle(150);
    await answerSummary(0, sv('sv2', 'd2'));
    const hash = JSON.stringify(queryKeys.detail(task(1), 'document'));
    adapter.streamCalls[0]?.push(
      ...taskStream(1, { document: 'd2' }, { text: 'half' }).slice(0, 3),
    );
    await settle(0);
    expect(core.details.partials.get(hash)).toMatchObject({ blocks: [{ text: 'half' }] });
    expect(sectionOf(core.queryClient, task(1), 'document').version).toBe('d1');
  });
});

describe('the presentation gate', () => {
  it('holds the reads, and makes one after it opens', async () => {
    openTask();
    const release = core.gate.acquire('drawer');
    note('history');
    note('history');
    await settle(20_000);
    expect(adapter.summaryCalls).toHaveLength(0);
    expect(adapter.streamCalls).toHaveLength(0);
    release();
    await settle(3_150);
    expect(adapter.summaryCalls).toHaveLength(1);
    await answerSummary(0, sv('sv2', 'd1', 'h2'));
    expect(sectionsAsked()).toEqual(['history']);
  });

  it('stops a background read that is running when something opens, and makes it again after', async () => {
    openTask();
    note('document');
    await settle(150);
    await answerSummary(0, sv('sv2', 'd2'));
    const first = adapter.streamCalls[0];
    expect(first?.signal.aborted).toBe(false);
    const release = core.gate.acquire('drawer');
    expect(first?.signal.aborted).toBe(true);
    await settle(20_000);
    expect(adapter.streamCalls).toHaveLength(1);
    expect(sectionOf(core.queryClient, task(1), 'document').version).toBe('d1');
    release();
    await settle(3_150);
    expect(sectionsAsked()).toEqual(['document', 'document']);
    adapter.streamCalls[1]?.respond(taskStream(1, { document: 'd2' }, { text: 'new' }));
    await settle(0);
    expect(sectionOf(core.queryClient, task(1), 'document').version).toBe('d2');
  });

  it('does not hold back a decision being asked, nor what a person asked for', async () => {
    openTask();
    core.gate.acquire('drawer');
    note('summary', 'request', 'document');
    expect(adapter.summaryCalls).toHaveLength(1);
    await answerSummary(0, sv('sv2', 'd2'));
    expect(sectionsAsked()).toEqual(['document']);
    const refresh = core.details.fetch(task(1), ['history']);
    await settle(0);
    expect(sectionsAsked()).toEqual(['document', 'history']);
    adapter.streamCalls[1]?.respond(taskStream(1, { history: 'h1' }));
    await refresh;
  });
});

describe('a reset', () => {
  it('forces a fresh read for a read that began before it', async () => {
    openTask();
    note('document');
    await settle(150);
    await answerSummary(0, sv('sv2', 'd2'));
    const old = adapter.streamCalls[0];
    expect(old?.signal.aborted).toBe(false);
    // A notification is missed: nothing can be told of what the running read will return.
    adapter.handlers?.onChanged(
      changed(seq + 5, [{ resource: 'project', id: P, facets: ['summary'] }]),
    );
    await settle(150);
    await answerSummary(1, sv('sv3', 'd2'));
    expect(old?.signal.aborted).toBe(true);
    expect(adapter.streamCalls).toHaveLength(2);
    adapter.streamCalls[1]?.respond(taskStream(1, { document: 'd2' }, { text: 'fresh' }));
    await settle(0);
    expect(sectionOf(core.queryClient, task(1), 'document')).toMatchObject({
      version: 'd2',
      invalidated: false,
    });
  });

  it('trusts a whole version the new Summary confirms, and reads nothing', async () => {
    openTask();
    note('summary');
    adapter.handlers?.onChanged(
      changed(seq + 5, [{ resource: 'project', id: P, facets: ['summary'] }]),
    );
    expect(sectionOf(core.queryClient, task(1), 'document').invalidated).toBe(true);
    await settle(150);
    await answerSummary(0, sv('sv2', 'd1'));
    expect(sectionOf(core.queryClient, task(1), 'document').invalidated).toBe(false);
    expect(adapter.streamCalls).toHaveLength(0);
  });
});

describe('Retry', () => {
  it('shows the error at once, with no hidden retries', async () => {
    openTask({ retry: true });
    const close = showDetail(core.queryClient, core.details, task(2), ['history']);
    // The first read retries on its own, as a background read does.
    await settle(0);
    for (let i = 0; i < 3; i++) {
      adapter.streamCalls[i]?.fail(new Error('boom'));
      await settle(5_000);
    }
    expect(adapter.streamCalls).toHaveLength(3);
    expect(sectionOf(core.queryClient, task(2), 'history').status).toBe('error');
    // Retry: one request, and its failure is the answer.
    const retry = core.details.fetch(task(2), ['history']);
    await settle(0);
    expect(adapter.streamCalls).toHaveLength(4);
    adapter.streamCalls[3]?.fail(new Error('boom again'));
    await retry;
    await settle(0);
    expect(adapter.streamCalls).toHaveLength(4);
    expect(sectionOf(core.queryClient, task(2), 'history')).toMatchObject({
      status: 'error',
      fetching: false,
    });
    close();
  });
});

describe('refresh', () => {
  it('rejects when the read fails, though the older document is still cached', async () => {
    const close = openTask();
    const refresh = core.details.refresh(task(1), ['document']);
    const rejected = expect(refresh).rejects.toThrow('boom');
    await settle(0);
    adapter.streamCalls[0]?.fail(new Error('boom'));
    await rejected;
    // The query keeps what it held, which is why a caller cannot tell from the cache.
    expect(core.details.entry(task(1), 'document')?.version).toBe('d1');
    expect(sectionOf(core.queryClient, task(1), 'document').status).toBe('error');
    close();
  });

  it('does not settle with an older version it joined once the Summary names a newer one', async () => {
    const close = openTask();
    const refresh = core.details.refresh(task(1), ['document']);
    let settled = false;
    void refresh.then(() => {
      settled = true;
    });
    await settle(0);
    expect(adapter.streamCalls).toHaveLength(1);
    // The Summary now names a newer document while the older one is still being read.
    note('document');
    await settle(150);
    await answerSummary(0, sv('sv2', 'd2'));
    adapter.streamCalls[0]?.respond(taskStream(1, { document: 'd1' }, { text: 'old' }));
    await settle(0);
    expect(settled).toBe(false);
    expect(core.details.entry(task(1), 'document')?.version).toBe('d1');
    adapter.streamCalls[adapter.streamCalls.length - 1]?.respond(
      taskStream(1, { document: 'd2' }, { text: 'new' }),
    );
    await refresh;
    expect(core.details.entry(task(1), 'document')?.version).toBe('d2');
    close();
  });

  it('settles quietly through fetch, which leaves the failure in the query', async () => {
    const close = openTask();
    const fetched = core.details.fetch(task(1), ['document']);
    await settle(0);
    adapter.streamCalls[0]?.fail(new Error('boom'));
    await expect(fetched).resolves.toBeUndefined();
    close();
  });
});

describe('closing and opening again', () => {
  it('reads fresh a Detail that changed while its read was cut short', async () => {
    const close = openTask();
    note('document');
    await settle(150);
    await answerSummary(0, sv('sv2', 'd2'));
    expect(sectionsAsked()).toEqual(['document']);
    // The Detail is closed during the read: the read is cut, the earlier version stays.
    close();
    await settle(0);
    expect(adapter.streamCalls[0]?.signal.aborted).toBe(true);
    expect(sectionOf(core.queryClient, task(1), 'document').version).toBe('d1');
    // Opened again within the time the cache keeps it: what the Summary names is read.
    showDetail(core.queryClient, core.details, task(1));
    await settle(0);
    expect(sectionsAsked()).toEqual(['document', 'document']);
    adapter.streamCalls[1]?.respond(taskStream(1, { document: 'd2' }, { text: 'new' }));
    await settle(0);
    expect(sectionOf(core.queryClient, task(1), 'document')).toMatchObject({
      version: 'd2',
      invalidated: false,
    });
  });

  it('marks a Detail that changed while it was closed, and reads it when opened', async () => {
    const close = openTask();
    close();
    note('history');
    await settle(150);
    await answerSummary(0, sv('sv2', 'd1', 'h2'));
    expect(adapter.streamCalls).toHaveLength(0); // nobody shows it
    expect(sectionOf(core.queryClient, task(1), 'history').invalidated).toBe(true);
    showDetail(core.queryClient, core.details, task(1));
    await settle(0);
    expect(sectionsAsked()).toEqual(['history']);
  });
});

describe('what the Summary does not list', () => {
  it('keeps what a screen shows, and drops what nothing shows', async () => {
    const close = openTask();
    seedDetail(core.queryClient, task(2), 'document', 'x1');
    note('summary');
    await settle(150);
    await answerSummary(0, sv('sv2', 'd1'));
    expect(sectionOf(core.queryClient, task(2), 'document').status).toBe('absent');
    expect(sectionOf(core.queryClient, task(1), 'document').version).toBe('d1');
    close();
  });
});
