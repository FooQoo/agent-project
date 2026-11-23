import { useCallback, useState } from 'react';
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
  known,
  disabled,
  onPending,
}: PlanFieldArgs & { projectId: ProjectId; taskId: number; known?: string | null }) {
  const { versions, release, tasks, refresh } = useProjectVersions(projectId);
  // An archived task is not in the Summary: the version the dialog opened with stands in for it,
  // and a save replaces it with the version the server stored.
  const [saved, setSaved] = useState<string | null | undefined>(undefined);
  const listed = tasks.find((task) => task.id === taskId);
  const planned = listed ? listed.plannedVersion : saved !== undefined ? saved : (known ?? null);
  const save = useCallback(
    async (choice: VersionChoice, expected: string | null) => {
      const result = await setTaskPlannedVersion(projectId, taskId, choice, expected);
      setSaved(result.task.plannedVersion ?? null);
      await refresh();
    },
    [projectId, taskId, refresh],
  );
  return (
    <PlannedVersionPicker
      taskId={taskId}
      planned={planned}
      release={release}
      versions={versions}
      disabled={disabled}
      onSave={save}
      onRefetch={() => void refresh()}
      onPending={onPending}
    />
  );
}
