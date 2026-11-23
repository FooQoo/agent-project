import { describe, expect, it } from 'vitest';
import { parseModeEvent, parseProjectMode } from './model';

describe('parseProjectMode', () => {
  it('reads a mode and its time', () => {
    expect(parseProjectMode({ mode: 'sleep', since: '2026-11-24T01:00:00Z' })).toEqual({
      mode: 'sleep',
      since: '2026-11-24T01:00:00Z',
    });
    expect(parseProjectMode({ mode: 'sparring' })).toEqual({ mode: 'sparring', since: null });
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
    expect(parseModeEvent(data)).toEqual([['p1', { mode: 'sleep', since: null }]]);
  });

  it('ignores unreadable data', () => {
    expect(parseModeEvent('not json')).toEqual([]);
    expect(parseModeEvent('{}')).toEqual([]);
  });
});
