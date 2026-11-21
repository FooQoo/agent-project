import { afterEach, describe, expect, it, vi } from 'vitest';
import { WriteError } from '../../../shared/api/writeErrors';
import {
  PlannedVersionChangedError,
  addVersion,
  setCurrentVersion,
  setEpicPlannedVersion,
  setTaskPlannedVersion,
} from './versions';

function answer(status: number, body: unknown) {
  const fetchMock = vi.fn(async () => ({
    ok: status < 400,
    status,
    json: async () => body,
  }));
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}

function sent(fetchMock: ReturnType<typeof answer>) {
  const [path, init] = fetchMock.mock.calls[0] as unknown as [string, RequestInit];
  return { path, method: init.method, body: JSON.parse(String(init.body)) as unknown };
}

afterEach(() => vi.unstubAllGlobals());

const PROJECT = '0123456789abcdef0123456789abcdef';

describe('version writes', () => {
  it('lists a version and sets the current one', async () => {
    const add = answer(200, { versions: ['v1.9.0'], currentVersion: null, changed: true });
    await addVersion(PROJECT, 'v1.9.0');
    expect(sent(add)).toMatchObject({ method: 'POST', body: { name: 'v1.9.0' } });
    expect(sent(add).path).toContain('/versions');

    const current = answer(200, { versions: ['v1.9.0'], currentVersion: 'v1.9.0', changed: true });
    await setCurrentVersion(PROJECT, 'v1.9.0');
    expect(sent(current)).toMatchObject({ method: 'PUT', body: { version: 'v1.9.0' } });
    expect(sent(current).path).toContain('/versions/current');
  });

  it('sends the page expectation with a task choice, and only when given', async () => {
    const withExpected = answer(200, { task: {}, changed: true });
    await setTaskPlannedVersion(PROJECT, 7, { choice: 'named', version: 'v2' }, null);
    expect(sent(withExpected).body).toEqual({
      choice: 'named',
      version: 'v2',
      expectedPlannedVersion: null,
    });
    expect(sent(withExpected).path).toContain('/tasks/7/planned-version');

    const without = answer(200, { task: {}, changed: true });
    await setTaskPlannedVersion(PROJECT, 7, { choice: 'current' });
    expect(sent(without).body).toEqual({ choice: 'current' });
  });

  it('names the version a stale task has now', async () => {
    answer(409, {
      error: { code: 'planned_version_changed', message: 'm', reason: 'changed' },
      currentPlannedVersion: 'v3',
    });
    const failure = await setTaskPlannedVersion(PROJECT, 7, { choice: 'unset' }, 'v1').catch(
      (e: unknown) => e,
    );
    expect(failure).toBeInstanceOf(PlannedVersionChangedError);
    expect((failure as PlannedVersionChangedError).current).toBe('v3');
  });

  it('keeps other refusals as write errors', async () => {
    answer(422, { error: { code: 'invalid_argument', message: 'm', reason: 'no such version' } });
    const failure = await setTaskPlannedVersion(PROJECT, 7, {
      choice: 'named',
      version: 'x',
    }).catch((e: unknown) => e);
    expect(failure).toBeInstanceOf(WriteError);
  });

  it('previews and applies an epic-wide change with the checked tasks', async () => {
    const preview = answer(200, {
      changed: [1],
      unchanged: [],
      skipped: [2],
      skippedReasons: { '2': 'done' },
      dryRun: true,
    });
    const result = await setEpicPlannedVersion(PROJECT, 'PLAN', {
      choice: { choice: 'named', version: 'v2' },
      scope: 'unplanned-only',
      taskIds: [1, 2],
      dryRun: true,
    });
    expect(result.skippedReasons['2']).toBe('done');
    expect(sent(preview).body).toEqual({
      choice: 'named',
      version: 'v2',
      scope: 'unplanned-only',
      taskIds: [1, 2],
      dryRun: true,
    });
    expect(sent(preview).path).toContain('/epics/PLAN/planned-version');
  });
});
