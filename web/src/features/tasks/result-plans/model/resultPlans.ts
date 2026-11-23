import type { ResultPlan, ResultPlanView, Task } from '../../../../shared/api/contracts';

/** One plan as the editor holds it. A plan that is not saved yet has a temporary `new-<n>` ID. */
export interface ResultPlanItem {
  id: string;
  content: string;
  condition: string;
  final: boolean;
  /** The version of the result that delivered it; `null` when not delivered. Never sent. */
  delivered: number | null;
}

export function isNewPlan(item: Pick<ResultPlanItem, 'id'>): boolean {
  return item.id.startsWith('new-');
}

export function itemOf(plan: ResultPlanView): ResultPlanItem {
  return {
    id: plan.id,
    content: plan.content,
    condition: plan.condition,
    final: plan.final === true,
    delivered: plan.deliveredResultVersion,
  };
}

/** The plans an editor shows for a card; none when the stored list cannot be read. */
export function plansOf(task: Pick<Task, 'resultPlans' | 'resultPlansError'>): ResultPlanItem[] {
  if (task.resultPlansError) return [];
  return (task.resultPlans ?? []).map(itemOf);
}

export function nextTempId(items: readonly Pick<ResultPlanItem, 'id'>[]): string {
  let n = 1;
  while (items.some((item) => item.id === `new-${n}`)) n += 1;
  return `new-${n}`;
}

/** What the server takes: new plans leave their `id` out; the text is trimmed. */
export function toWire(items: readonly ResultPlanItem[]): (Omit<ResultPlan, 'id'> & {
  id?: string;
})[] {
  return items.map((item) => ({
    ...(isNewPlan(item) ? {} : { id: item.id }),
    content: item.content.trim(),
    condition: item.condition.trim(),
    final: item.final,
  }));
}

export type PlanProblem = { index: number; kind: 'empty' } | { kind: 'finalMany' };

/** The first thing that stops a save: an empty field, or more than one final plan. */
export function planProblem(items: readonly ResultPlanItem[]): PlanProblem | null {
  const empty = items.findIndex(
    (item) => item.content.trim() === '' || item.condition.trim() === '',
  );
  if (empty >= 0) return { index: empty, kind: 'empty' };
  if (items.filter((item) => item.final).length > 1) return { kind: 'finalMany' };
  return null;
}

export function deliveredCount(
  plans: readonly { deliveredResultVersion: number | null }[],
): number {
  return plans.filter((plan) => plan.deliveredResultVersion !== null).length;
}
