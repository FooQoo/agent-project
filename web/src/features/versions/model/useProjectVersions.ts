import { useSummary } from '../../../shared/data/react';
import type { SummaryEpic, SummaryTask } from '../../../shared/data/api';
import type { ProjectId } from '../../../shared/api/contracts';

export interface ProjectVersionState {
  /** The versions the project lists. */
  versions: readonly string[];
  /** The project's current version; `null` when none is set. */
  current: string | null;
  /** The project's tasks, as its Summary lists them. */
  tasks: readonly SummaryTask[];
  /** The project's epics. */
  epics: readonly SummaryEpic[];
  /** Reads the Summary again now, without the presentation hold. */
  refresh: () => Promise<void>;
}

const NONE: readonly string[] = [];
const NO_TASKS: readonly SummaryTask[] = [];
const NO_EPICS: readonly SummaryEpic[] = [];

/** The project's versions and current version, from its Summary. */
export function useProjectVersions(projectId: ProjectId | null): ProjectVersionState {
  const { summary, refresh } = useSummary(projectId);
  return {
    versions: summary?.project.versions ?? NONE,
    current: summary?.project.currentVersion ?? null,
    tasks: summary?.tasks ?? NO_TASKS,
    epics: summary?.epics ?? NO_EPICS,
    refresh: async () => {
      await refresh();
    },
  };
}
