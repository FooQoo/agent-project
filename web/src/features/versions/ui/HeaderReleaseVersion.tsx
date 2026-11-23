import type { ProjectId } from '../../../shared/api/contracts';
import { addVersion, setReleaseVersion } from '../api/versions';
import { useProjectVersions } from '../model/useProjectVersions';
import { ReleaseVersionMenu } from './ReleaseVersionMenu';

/** The header's release-version menu of a project, wired to the version routes. */
export function HeaderReleaseVersion({
  projectId,
  disabled,
}: {
  projectId: ProjectId;
  disabled?: boolean;
}) {
  const { versions, release, refresh } = useProjectVersions(projectId);
  return (
    <ReleaseVersionMenu
      versions={versions}
      release={release}
      disabled={disabled}
      onSelect={async (version) => {
        await setReleaseVersion(projectId, version);
        await refresh();
      }}
      onAdd={async (name) => {
        await addVersion(projectId, name);
        await refresh();
      }}
    />
  );
}
