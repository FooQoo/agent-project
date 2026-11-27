import { describe, expect, it, vi } from 'vitest';
import {
  answerRapQuestion,
  approveRapPlan,
  authorizeRapDirect,
  createRapWork,
  enterRapSleep,
  loadRapResults,
  loadRapSummary,
  reviseRapResult,
  submitRapChangeRequest,
  subscribeRapEvents,
  wakeRapSleep,
} from './api';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

describe('RAP web API client', () => {
  it('calls summary, work, approval, interrupt, Q&A, results, and sleep routes', async () => {
    const calls: Array<{ url: string; method: string }> = [];
    const fetcher = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      calls.push({ url, method: init?.method ?? 'GET' });
      if (url.endsWith('/summary')) {
        return jsonResponse({
          sleeping: false,
          settings: { auto_run: false, max_sessions: 1 },
          items: [],
        });
      }
      if (url.endsWith('/results')) {
        return jsonResponse({ results: [], evidence: [] });
      }
      if (url.endsWith('/sleep/enter')) {
        return jsonResponse({ sleeping: true });
      }
      if (url.endsWith('/sleep/wake')) {
        return jsonResponse({
          sleep_started_at: 100,
          sleep_ended_at: 200,
          delegated_decisions: [],
          unanswered_questions: [],
          recorded_evidence: [],
        });
      }
      return jsonResponse({ id: 1, work_id: 1, state: 'pending' }, 201);
    }) as unknown as typeof fetch;

    const summary = await loadRapSummary('demo', fetcher);
    expect(summary.settings.auto_run).toBe(false);
    await createRapWork('demo', { workspace: '/repo', kind: 'task', title: 'Ship' }, fetcher);
    await approveRapPlan(1, 2, 'human', fetcher);
    await authorizeRapDirect(1, 'Do it', 'human', fetcher);
    await submitRapChangeRequest(1, 'Add check', fetcher);
    await answerRapQuestion(5, 'Yes', 'human', fetcher);
    await loadRapResults(1, fetcher);
    await reviseRapResult(7, 'Fix typo', fetcher);
    await enterRapSleep(fetcher);
    await wakeRapSleep(fetcher);

    expect(calls.map((c) => `${c.method} ${c.url}`)).toEqual([
      'GET /api/rap/projects/demo/summary',
      'POST /api/rap/projects/demo/work',
      'POST /api/rap/work/1/approve-plan',
      'POST /api/rap/work/1/authorize-direct',
      'POST /api/rap/work/1/change-requests',
      'POST /api/rap/questions/5/answer',
      'GET /api/rap/work/1/results',
      'POST /api/rap/results/7/revise',
      'POST /api/rap/sleep/enter',
      'POST /api/rap/sleep/wake',
    ]);
  });

  it('forwards SSE events and triggers resync on lag or disconnect', () => {
    const listeners = new Map<string, (e: Event) => void>();
    const fakeSource = {
      onerror: null as (() => void) | null,
      addEventListener: (type: string, cb: (e: Event) => void) => {
        listeners.set(type, cb);
      },
      close: vi.fn(),
    } as unknown as EventSource;

    const received: unknown[] = [];
    const onResync = vi.fn();
    const sub = subscribeRapEvents(
      (e) => received.push(e),
      onResync,
      () => fakeSource,
    );

    listeners.get('plan_approved')?.({
      data: JSON.stringify({ kind: 'plan_approved', work_id: 3, version: 2 }),
    } as MessageEvent);
    expect(received).toEqual([{ kind: 'plan_approved', work_id: 3, version: 2 }]);

    listeners.get('resync')?.({} as Event);
    fakeSource.onerror?.({} as Event);
    expect(onResync).toHaveBeenCalledTimes(2);

    sub.close();
    expect(fakeSource.close).toHaveBeenCalledTimes(1);
  });
});
