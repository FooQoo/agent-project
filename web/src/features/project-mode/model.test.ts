import { describe, expect, it } from 'vitest';
import { parseModeEvent, parseProjectMode, SPARRING, sameTarget, targetLabel } from './model';

describe('parseProjectMode', () => {
  it('reads a mode and its time', () => {
    expect(parseProjectMode({ mode: 'sleep', since: '2026-11-24T01:00:00Z' })).toEqual({
      ...SPARRING,
      mode: 'sleep',
      since: '2026-11-24T01:00:00Z',
    });
    expect(parseProjectMode({ mode: 'sparring' })).toEqual(SPARRING);
  });

  it('reads the source, the end of the window, the manual mode, and the schedule', () => {
    const schedule = { enabled: true, start: '23:00', end: '08:00' };
    expect(
      parseProjectMode({
        mode: 'sleep',
        modeSource: 'schedule',
        until: '2026-11-24T08:00:00Z',
        manual: 'sparring',
        schedule,
      }),
    ).toEqual({
      ...SPARRING,
      mode: 'sleep',
      modeSource: 'schedule',
      until: '2026-11-24T08:00:00Z',
      manual: 'sparring',
      schedule,
    });
    // A schedule that is not two HH:MM times is dropped, not half-read.
    expect(
      parseProjectMode({
        mode: 'sparring',
        schedule: { enabled: true, start: '8:00', end: '09:00' },
      })?.schedule,
    ).toBeNull();
  });

  it('reads a brainstorm with its target', () => {
    const target = { type: 'task', id: '680' };
    expect(parseProjectMode({ mode: 'brainstorm', manual: 'brainstorm', target })).toEqual({
      ...SPARRING,
      mode: 'brainstorm',
      manual: 'brainstorm',
      target,
    });
    // The target is read only while brainstorming.
    expect(parseProjectMode({ mode: 'sleep', target })?.target).toBeNull();
  });

  it('refuses a brainstorm without a readable target', () => {
    expect(parseProjectMode({ mode: 'brainstorm' })).toBeNull();
    expect(parseProjectMode({ mode: 'brainstorm', target: { type: 'other', id: '1' } })).toBeNull();
    expect(parseProjectMode({ mode: 'brainstorm', target: { type: 'task', id: 1 } })).toBeNull();
  });

  it('refuses an unknown mode or a wrong shape', () => {
    expect(parseProjectMode({ mode: 'nap' })).toBeNull();
    expect(parseProjectMode(null)).toBeNull();
    expect(parseProjectMode('sleep')).toBeNull();
  });
});

describe('parseModeEvent', () => {
  it('lists the projects that changed', () => {
    const data = JSON.stringify({ projects: { p1: { mode: 'sleep' }, p2: { mode: 'x' } } });
    expect(parseModeEvent(data)).toEqual([['p1', { ...SPARRING, mode: 'sleep' }]]);
  });

  it('ignores unreadable data', () => {
    expect(parseModeEvent('not json')).toEqual([]);
    expect(parseModeEvent('{}')).toEqual([]);
  });
});

describe('targetLabel and sameTarget', () => {
  it('names a task by its number and an epic by its ID', () => {
    expect(targetLabel({ type: 'task', id: '680' })).toBe('#680');
    expect(targetLabel({ type: 'epic', id: 'INTERACTION-MODES' })).toBe('INTERACTION-MODES');
  });

  it('tells targets apart by type and ID', () => {
    const task = { type: 'task', id: '1' } as const;
    expect(sameTarget(task, { type: 'task', id: '1' })).toBe(true);
    expect(sameTarget(task, { type: 'epic', id: '1' })).toBe(false);
    expect(sameTarget(null, task)).toBe(false);
  });
});

describe('held questions', () => {
  it('reads the held count only while asleep', () => {
    const base = { mode: 'sleep', modeSource: 'manual', heldCount: 3 };
    expect(parseProjectMode(base)?.heldCount).toBe(3);
    expect(parseProjectMode({ ...base, mode: 'sparring' })?.heldCount).toBe(0);
    expect(parseProjectMode({ ...base, heldCount: -1 })?.heldCount).toBe(0);
    expect(parseProjectMode({ ...base, heldCount: 'x' })?.heldCount).toBe(0);
  });
});
