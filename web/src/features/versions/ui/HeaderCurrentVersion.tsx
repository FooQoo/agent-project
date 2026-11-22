import type { ProjectId } from '../../../shared/api/contracts';
import { addVersion, setCurrentVersion } from '../api/versions';
import { useProjectVersions } from '../model/useProjectVersions';
import { CurrentVersionMenu } from './CurrentVersionMenu';

/** The header's current-version menu of a project, wired to the version routes. */
export function HeaderCurrentVersion({
  projectId,
  disabled,
}: {
  projectId: ProjectId;
  disabled?: boolean;
}) {
  const { versions, current, refresh } = useProjectVersions(projectId);
  return (
    <CurrentVersionMenu
      versions={versions}
      current={current}
      disabled={disabled}
      onSelect={async (version) => {
        await setCurrentVersion(projectId, version);
        await refresh();
      }}
      onAdd={async (name) => {
        await addVersion(projectId, name);
        await refresh();
      }}
    />
  );
}
