import { useCallback } from 'react';
import { useWrite } from '../../../shared/data/react/use-write';
import { archiveTask, archiveDone, deleteTask, restoreTask } from '../api/lifecycle';
import type { OperationControls, OperationKind } from '../../../shared/lib/operations/operations';
import type { ProjectId } from '../../../shared/api/contracts';

export type TaskLifecycleAction = 'archive' | 'restore' | 'delete' | 'archive-done';
export type TaskLifecycleTarget = { taskId: number } | { taskIds: readonly number[] };
export type TaskLifecycleResult = { ok: true } | { ok: false; error: unknown };

function operationKind(action: TaskLifecycleAction): OperationKind {
  if (action === 'archive-done') return 'archive-done';
  return action;
}

async function perform(
  action: TaskLifecycleAction,
  projectId: ProjectId,
  target: TaskLifecycleTarget,
): Promise<void> {
  if (action === 'archive') {
    if (!('taskId' in target)) throw new Error('A card ID is required to archive a card.');
    await archiveTask(projectId, target.taskId);
  } else if (action === 'restore') {
    if (!('taskId' in target)) throw new Error('A card ID is required to restore a card.');
    await restoreTask(projectId, target.taskId);
  } else if (action === 'delete') {
    if (!('taskId' in target)) throw new Error('A card ID is required to delete a card.');
    const result = await deleteTask(projectId, target.taskId);
    if (result === 'failed') throw new Error('The card could not be deleted.');
  } else {
    if (!('taskIds' in target)) throw new Error('Task IDs are required for bulk archive.');
    await archiveDone(projectId, target.taskIds);
  }
}

const KEY = ['task-lifecycle'] as const;

/**
 * Executes existing card APIs inside the project session's operation. One mutation is on its way
 * at a time: a second request is refused while it runs.
 */
export function useTaskLifecycle(operations: OperationControls) {
  const mutation = useWrite<
    void,
    { action: TaskLifecycleAction; projectId: ProjectId; target: TaskLifecycleTarget }
  >({
    mutationKey: KEY,
    mutationFn: ({ action, projectId, target }) => perform(action, projectId, target),
  });
  const { mutateAsync, isWriting } = mutation;

  const run = useCallback(
    async (
      action: TaskLifecycleAction,
      projectId: ProjectId,
      target: TaskLifecycleTarget,
      options: { keepOperationOnError?: boolean } = {},
    ): Promise<TaskLifecycleResult> => {
      if (isWriting()) return { ok: false, error: new Error('An operation is already running.') };
      const operationId = operations.begin(operationKind(action), projectId);
      if (operations.isCurrent && !operations.isCurrent(operationId, projectId))
        return { ok: false, error: new Error('The project operation is no longer current.') };
      operations.saveStarted(operationId);
      try {
        await mutateAsync({ action, projectId, target });
        if (!operations.isCurrent || operations.isCurrent(operationId, projectId)) {
          if (action !== 'archive-done' && 'taskId' in target && operations.taskRemoved) {
            operations.taskRemoved(operationId, projectId, target.taskId);
          } else {
            operations.end(operationId);
          }
        }
        return { ok: true };
      } catch (error) {
        if (!operations.isCurrent || operations.isCurrent(operationId, projectId)) {
          operations.saveFailed(operationId);
          if (!options.keepOperationOnError) operations.end(operationId);
        }
        return { ok: false, error };
      }
    },
    [operations, mutateAsync, isWriting],
  );

  return { run };
}
