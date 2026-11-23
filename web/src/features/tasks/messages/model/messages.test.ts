import { describe, expect, it } from 'vitest';
import { brainstormLabel, parseMessageDetail } from './messages';

const question = (extra: Record<string, unknown>) => ({
  question: { id: 'Q-1', body: 'Who reads this?', status: 'open', ...extra },
});

describe('parseMessageDetail brainstorm', () => {
  it('reads the target a brainstorm question is about', () => {
    expect(
      parseMessageDetail(question({ brainstorm: { type: 'task', id: '680' } })).brainstorm,
    ).toEqual({ type: 'task', id: '680' });
  });

  it('is null for an ordinary question or a target it cannot read', () => {
    expect(parseMessageDetail(question({ brainstorm: null })).brainstorm).toBeNull();
    expect(parseMessageDetail(question({})).brainstorm).toBeNull();
    expect(
      parseMessageDetail(question({ brainstorm: { type: 'other', id: '1' } })).brainstorm,
    ).toBeNull();
  });
});

describe('brainstormLabel', () => {
  it('names a task by number and an epic by ID', () => {
    expect(brainstormLabel({ type: 'task', id: '680' })).toBe('#680');
    expect(brainstormLabel({ type: 'epic', id: 'INTERACTION-MODES' })).toBe('INTERACTION-MODES');
  });
});
