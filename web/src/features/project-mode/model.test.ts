import { describe, expect, it } from 'vitest';
import { parseModeEvent, parseProjectMode, SPARRING } from './model';

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

  it('refuses an unknown mode or a wrong shape', () => {
    expect(parseProjectMode({ mode: 'brainstorm' })).toBeNull();
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
