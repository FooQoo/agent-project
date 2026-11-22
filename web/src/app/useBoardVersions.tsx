import { HeaderCurrentVersion } from '../features/versions/ui/HeaderCurrentVersion';
import { VersionTag, showsVersionTag } from '../features/versions/ui/VersionTag';
import type { TaskSummary } from '../shared/api/contracts';
import type { AppControllerModel } from './contracts';
import { isProjectId } from './model/projectSession';

const NO_VERSIONS: readonly string[] = [];

/**
 * The PC board's planned versions: the tag of a card (in its own slot, apart from the epic label)
 * and the header's current-version menu. Nothing else of the board changes with them.
 */
export function useBoardVersions(model: AppControllerModel) {
  const { boardData, activeId } = model;
  // A sample board of a Story may predate the version fields.
  const versions = boardData?.project?.versions ?? NO_VERSIONS;
  const current = boardData?.project?.currentVersion ?? null;
  const renderVersion = (task: TaskSummary) =>
    showsVersionTag(task.plannedVersion ?? null, { versions, current }) ? (
      <VersionTag planned={task.plannedVersion ?? null} current={current} />
    ) : null;
  const versionSlot =
    boardData !== null && isProjectId(activeId) ? (
      <HeaderCurrentVersion projectId={activeId} disabled={model.session.operation !== null} />
    ) : null;
  return { renderVersion, versionSlot };
}
