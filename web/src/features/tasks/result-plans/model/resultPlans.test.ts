import { describe, expect, it } from 'vitest';
import {
  deliveredCount,
  isNewPlan,
  itemOf,
  nextTempId,
  planProblem,
  plansOf,
  toWire,
  type ResultPlanItem,
} from './resultPlans';

const item = (over: Partial<ResultPlanItem> = {}): ResultPlanItem => ({
  id: 'RP-1',
  content: 'A screenshot',
  condition: 'When fixed',
  final: false,
  delivered: null,
  ...over,
});

describe('result plan items', () => {
  it('reads the delivered state from the task', () => {
    const plans = plansOf({
      resultPlans: [
        { id: 'RP-1', content: 'A', condition: 'B', deliveredResultVersion: 2 },
        { id: 'RP-2', content: 'C', condition: 'D', final: true, deliveredResultVersion: null },
      ],
    });
    expect(plans.map((p) => [p.id, p.final, p.delivered])).toEqual([
      ['RP-1', false, 2],
      ['RP-2', true, null],
    ]);
    expect(deliveredCount([{ deliveredResultVersion: 2 }, { deliveredResultVersion: null }])).toBe(
      1,
    );
  });

  it('offers no plans for a list that cannot be read', () => {
    expect(plansOf({ resultPlans: [], resultPlansError: true })).toEqual([]);
    expect(plansOf({})).toEqual([]);
    expect(
      itemOf({ id: 'RP-3', content: 'x', condition: 'y', deliveredResultVersion: null }).final,
    ).toBe(false);
  });

  it('leaves the id of a new plan out and trims the text', () => {
    const wire = toWire([
      item(),
      item({ id: 'new-1', content: ' Report ', condition: ' Done ', final: true }),
    ]);
    expect(wire[0]).toEqual({
      id: 'RP-1',
      content: 'A screenshot',
      condition: 'When fixed',
      final: false,
    });
    expect(wire[1]).toEqual({ content: 'Report', condition: 'Done', final: true });
    expect(isNewPlan({ id: 'new-1' })).toBe(true);
    expect(nextTempId([{ id: 'new-1' }, { id: 'RP-1' }])).toBe('new-2');
  });

  it('stops a save on an empty field or two final plans', () => {
    expect(planProblem([item()])).toBeNull();
    expect(planProblem([item(), item({ id: 'RP-2', condition: ' ' })])).toEqual({
      index: 1,
      kind: 'empty',
    });
    expect(planProblem([item({ final: true }), item({ id: 'RP-2', final: true })])).toEqual({
      kind: 'finalMany',
    });
  });
});
