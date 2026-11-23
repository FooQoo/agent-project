import { describe, expect, it } from 'vitest';
import { filterByVersion } from './versionFilter';

const tasks = [
  { id: 1, taskStatus: 'todo' as const, plannedVersion: 'v1.0.0' },
  { id: 2, taskStatus: 'doing' as const, plannedVersion: 'v2.0.0' },
  { id: 3, taskStatus: 'done' as const, plannedVersion: null },
  { id: 4, taskStatus: 'todo' as const },
  { id: 5, taskStatus: 'backlog' as const, plannedVersion: 'v1.0.0' },
];

describe('filterByVersion', () => {
  it('shows every card for All', () => {
    expect(filterByVersion(tasks, null).map((task) => task.id)).toEqual([1, 2, 3, 4, 5]);
  });

  it('keeps the version, the unplanned cards and the whole Backlog', () => {
    expect(filterByVersion(tasks, 'v2.0.0').map((task) => task.id)).toEqual([2, 3, 4, 5]);
  });
});
