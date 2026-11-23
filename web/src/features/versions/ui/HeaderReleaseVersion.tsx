import type { ProjectId } from '../../../shared/api/contracts';
import { addVersion, setReleaseVersion } from '../api/versions';
import { useProjectVersions } from '../model/useProjectVersions';
import { ReleaseVersionMenu } from './ReleaseVersionMenu';

/** The header's version filter of a project, wired to the version routes. */
export function HeaderReleaseVersion({
  projectId,
  filter,
  onFilter,
  disabled,
}: {
  projectId: ProjectId;
  filter: string | null;
  onFilter: (version: string | null) => void;
  disabled?: boolean;
}) {
  const { versions, release, refresh } = useProjectVersions(projectId);
  return (
    <ReleaseVersionMenu
      versions={versions}
      release={release}
      filter={filter}
      onFilter={onFilter}
      disabled={disabled}
      onMakeRelease={async (version) => {
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
