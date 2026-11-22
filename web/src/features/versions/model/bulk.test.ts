import { describe, expect, it } from 'vitest';
import type { EpicVersionResult } from '../api/versions';
import { bulkRows, checkedIds, choiceOf, targetName } from './bulk';

const members = [
  { id: 1, title: 'one', taskStatus: 'todo', plannedVersion: null },
  { id: 2, title: 'two', taskStatus: 'todo', plannedVersion: 'v1.9.0' },
  { id: 3, title: 'three', taskStatus: 'done', plannedVersion: null },
  { id: 4, title: 'four', taskStatus: 'doing', plannedVersion: 'v1.8.0' },
];

describe('the epic bulk rows', () => {
  const result: EpicVersionResult = {
    changed: [1],
    unchanged: [4],
    skipped: [2, 3],
    skippedReasons: { '2': 'planned', '3': 'done' },
    dryRun: true,
  };

  it('shows what the server would do with each task', () => {
    const rows = bulkRows(members, result);
    expect(rows.map((row) => row.state)).toEqual(['change', 'skipped', 'skipped', 'same']);
    expect(rows[1]).toMatchObject({ reason: 'planned' });
    expect(rows[2]).toMatchObject({ reason: 'done' });
  });

  it('sends only the rows that change and stay checked', () => {
    const rows = bulkRows(members, result);
    expect(checkedIds(rows, new Set())).toEqual([1]);
    expect(checkedIds(rows, new Set([1]))).toEqual([]);
  });

  it('names the target version', () => {
    expect(targetName({ kind: 'named', version: 'v2' }, 'v1')).toBe('v2');
    expect(targetName({ kind: 'current' }, 'v1')).toBe('v1');
    expect(targetName({ kind: 'current' }, null)).toBeNull();
    expect(targetName({ kind: 'unset' }, 'v1')).toBeNull();
    expect(choiceOf({ kind: 'current' })).toEqual({ choice: 'current' });
    expect(choiceOf({ kind: 'unset' })).toEqual({ choice: 'unset' });
    expect(choiceOf({ kind: 'named', version: 'v2' })).toEqual({ choice: 'named', version: 'v2' });
  });
});
