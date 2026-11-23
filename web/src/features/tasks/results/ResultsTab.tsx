import { useCallback } from 'react';
import type { ProjectId, ResultPlanView } from '../../../shared/api/contracts';
import { ResultsView } from './ResultsView';
import { useTaskResults } from './useTaskResults';
import { deliverPlan, type PlanDelivery } from '../result-plans/deliverPlan';

/**
 * The results tab of one task: reads the results while the tab is shown and draws them. `refreshKey`
 * (the number of history entries) reads them again when a delivery lands.
 */
export function ResultsTab({
  projectId,
  taskId,
  active,
  refreshKey,
  attachmentUrl,
  focusVersion,
  canRegister = true,
}: {
  projectId: ProjectId;
  taskId: number;
  active: boolean;
  refreshKey: unknown;
  attachmentUrl: (attachmentId: string) => string;
  focusVersion?: number | null;
  /** The person may register results: not for a draft. */
  canRegister?: boolean;
}) {
  const results = useTaskResults(projectId, taskId, refreshKey, active);
  const { retry } = results;
  const register = useCallback(
    async (plan: ResultPlanView, delivery: PlanDelivery) => {
      await deliverPlan(projectId, taskId, plan, delivery);
      retry();
    },
    [projectId, taskId, retry],
  );
  return (
    <ResultsView
      state={results}
      refreshFailed={results.refreshFailed}
      attachmentUrl={attachmentUrl}
      focusVersion={focusVersion}
      onRetry={results.retry}
      canRegister={canRegister}
      onRegister={register}
    />
  );
}
