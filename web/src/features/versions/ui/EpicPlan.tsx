import { useMemo } from 'react';
import type { ProjectId } from '../../../shared/api/contracts';
import type { PlanFieldArgs } from '../../../shared/ui/editor/planField';
import { setEpicPlannedVersion } from '../api/versions';
import { useProjectVersions } from '../model/useProjectVersions';
import { EpicPlanField } from './EpicPlanField';

/**
 * An epic's bulk planned-version change in its editor, wired to the project's Summary and the
 * version routes. The preview is the server's own dry run; the board is read again after the save.
 */
export function EpicPlan({
  projectId,
  epicId,
  disabled,
  onPending,
}: PlanFieldArgs & { projectId: ProjectId; epicId: string }) {
  const { versions, current, tasks, refresh } = useProjectVersions(projectId);
  const members = useMemo(
    () =>
      tasks
        .filter((task) => task.epicId === epicId)
        .map((task) => ({
          id: task.id,
          title: task.title,
          taskStatus: task.taskStatus,
          plannedVersion: task.plannedVersion,
        })),
    [tasks, epicId],
  );
  return (
    <EpicPlanField
      members={members}
      current={current}
      versions={versions}
      disabled={disabled}
      preview={(choice, scope, taskIds) =>
        setEpicPlannedVersion(projectId, epicId, { choice, scope, taskIds, dryRun: true })
      }
      apply={async (choice, scope, taskIds) => {
        const result = await setEpicPlannedVersion(projectId, epicId, {
          choice,
          scope,
          taskIds,
          dryRun: false,
        });
        await refresh();
        return result;
      }}
      onPending={onPending}
    />
  );
}
