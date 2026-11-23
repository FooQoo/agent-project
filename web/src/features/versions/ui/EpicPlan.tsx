import { useMemo, useRef } from 'react';
import type { ProjectId } from '../../../shared/api/contracts';
import type { PlanFieldArgs } from '../../../shared/ui/editor/planField';
import { setEpicPlannedVersion } from '../api/versions';
import { useProjectVersions } from '../model/useProjectVersions';
import { EpicPlanField } from './EpicPlanField';

/**
 * An epic's planned version in its editor, wired to the project's Summary and the version routes.
 * Save plans every task of the epic that is not done, using the tasks the board lists at that time;
 * the board is read again afterwards.
 */
export function EpicPlan({
  projectId,
  epicId,
  disabled,
  onPending,
}: PlanFieldArgs & { projectId: ProjectId; epicId: string }) {
  const { versions, release, tasks, refresh } = useProjectVersions(projectId);
  const open = useMemo(
    () =>
      tasks.filter((task) => task.epicId === epicId && task.taskStatus !== 'done').map((t) => t.id),
    [tasks, epicId],
  );
  const openRef = useRef(open);
  openRef.current = open;
  return (
    <EpicPlanField
      release={release}
      versions={versions}
      disabled={disabled}
      apply={async (choice) => {
        const result = await setEpicPlannedVersion(projectId, epicId, {
          choice,
          scope: 'all',
          taskIds: openRef.current,
          dryRun: false,
        });
        await refresh();
        return result;
      }}
      onPending={onPending}
    />
  );
}
