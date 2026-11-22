import { useCallback } from 'react';
import type { ProjectId } from '../../../shared/api/contracts';
import type { PlanFieldArgs } from '../../../shared/ui/editor/planField';
import { type VersionChoice, setTaskPlannedVersion } from '../api/versions';
import { useProjectVersions } from '../model/useProjectVersions';
import { PlannedVersionPicker } from './PlannedVersionPicker';

/**
 * A task's planned version in its editor, wired to the project's versions. The editor saves it
 * last, over the value the Summary showed; the board is read again after a save or a conflict.
 */
export function TaskPlanField({
  projectId,
  taskId,
  disabled,
  onPending,
}: PlanFieldArgs & { projectId: ProjectId; taskId: number }) {
  const { versions, current, tasks, refresh } = useProjectVersions(projectId);
  const planned = tasks.find((task) => task.id === taskId)?.plannedVersion ?? null;
  const save = useCallback(
    async (choice: VersionChoice, expected: string | null) => {
      await setTaskPlannedVersion(projectId, taskId, choice, expected);
      await refresh();
    },
    [projectId, taskId, refresh],
  );
  return (
    <PlannedVersionPicker
      taskId={taskId}
      planned={planned}
      current={current}
      versions={versions}
      disabled={disabled}
      onSave={save}
      onRefetch={() => void refresh()}
      onPending={onPending}
    />
  );
}
