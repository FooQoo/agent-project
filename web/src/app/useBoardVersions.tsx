import { useState } from 'react';
import { filterByVersion } from '../features/versions/model/versionFilter';
import { HeaderReleaseVersion } from '../features/versions/ui/HeaderReleaseVersion';
import { VersionTag, showsVersionTag } from '../features/versions/ui/VersionTag';
import type { TaskSummary } from '../shared/api/contracts';
import type { AppControllerModel } from './contracts';
import { isProjectId } from './model/projectSession';

const NO_VERSIONS: readonly string[] = [];

/**
 * The PC board's planned versions: the tag of a card (in its own slot, apart from the epic label)
 * and the header's version filter, which hides the other versions' cards from the Todo, Doing and Done lanes.
 */
export function useBoardVersions(model: AppControllerModel) {
  const { boardData, activeId } = model;
  // A sample board of a Story may predate the version fields.
  const versions = boardData?.project?.versions ?? NO_VERSIONS;
  const release = boardData?.project?.releaseVersion ?? null;
  // The filter is this screen's own state: it starts as the project's release version (or All) until a choice is made.
  const [chosen, setChosen] = useState<{ project: string | null; version: string | null } | null>(
    null,
  );
  const filter = chosen !== null && chosen.project === activeId ? chosen.version : release;
  const filterTasks = (tasks: TaskSummary[]) => filterByVersion(tasks, filter);
  const renderVersion = (task: TaskSummary) =>
    showsVersionTag(task.plannedVersion ?? null, { versions, release }) ? (
      <VersionTag planned={task.plannedVersion ?? null} release={release} />
    ) : null;
  const versionSlot =
    boardData !== null && isProjectId(activeId) ? (
      <HeaderReleaseVersion
        projectId={activeId}
        filter={filter}
        onFilter={(version) => setChosen({ project: activeId, version })}
        disabled={model.session.operation !== null}
      />
    ) : null;
  return { renderVersion, versionSlot, filterTasks };
}
