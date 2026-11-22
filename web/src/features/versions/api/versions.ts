import { i18n } from '../../../shared/i18n';
import type { ApiError, ProjectId, Task } from '../../../shared/api/contracts';
import { authHeaders, projectPath } from '../../../shared/api/http';
import { WriteError, writeFailure } from '../../../shared/api/writeErrors';

/** The project's versions and the current one: `GET .../versions`. */
export interface ProjectVersions {
  versions: string[];
  currentVersion: string | null;
}

/** Which version to plan: a version the project lists, its current one, or none. */
export type VersionChoice =
  | { choice: 'named'; version: string }
  | { choice: 'current' }
  | { choice: 'unset' };

/** Which of the checked tasks an epic-wide change touches. */
export type EpicVersionScope = 'unplanned-only' | 'all';

/** Why the server left a task of an epic-wide change alone. */
export type SkipReason = 'archived' | 'done' | 'planned';

export interface EpicVersionResult {
  changed: number[];
  unchanged: number[];
  skipped: number[];
  /** The reason for each skipped task, by task ID. */
  skippedReasons: Record<string, SkipReason>;
  /** True when nothing was written: `changed` is what would change. */
  dryRun: boolean;
}

/** The task's planned version is no longer what the page showed; `current` is what it is now. */
export class PlannedVersionChangedError extends Error {
  constructor(
    message: string,
    readonly current: string | null,
  ) {
    super(message);
  }
}

async function send<T>(method: 'POST' | 'PUT', path: string, value: unknown): Promise<T> {
  const response = await fetch(path, {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...authHeaders(),
      'X-Agpj-Language': i18n.resolvedLanguage === 'ja' ? 'ja' : 'en',
    },
    body: JSON.stringify(value),
  });
  const body = (await response.json()) as { error?: ApiError['error'] };
  if (!response.ok) {
    throw writeFailure(
      body.error?.reason ?? `Request failed (${response.status})`,
      response.status,
      body.error?.code ?? null,
      body.error?.message ?? null,
      body,
    );
  }
  return body as T;
}

export async function readVersions(projectId: ProjectId): Promise<ProjectVersions> {
  const response = await fetch(`${projectPath(projectId)}/versions`, { headers: authHeaders() });
  if (!response.ok) throw new Error(`Request failed (${response.status})`);
  return (await response.json()) as ProjectVersions;
}

/** Lists a version. Listing one that is listed changes nothing (`changed: false`). */
export function addVersion(projectId: ProjectId, name: string) {
  return send<ProjectVersions & { changed: boolean }>(
    'POST',
    `${projectPath(projectId)}/versions`,
    {
      name,
    },
  );
}

/** Sets the current version, or clears it with `null`. A version not yet listed is listed. */
export function setCurrentVersion(projectId: ProjectId, version: string | null) {
  return send<ProjectVersions & { changed: boolean }>(
    'PUT',
    `${projectPath(projectId)}/versions/current`,
    { version },
  );
}

/**
 * Sets one task's planned version as a person. `expected` is the planned version the page showed
 * (`null` for none): a task that has another one now is refused, nothing is written, and
 * `PlannedVersionChangedError` names its current version. Without `expected` the choice is saved
 * over the latest file.
 */
export async function setTaskPlannedVersion(
  projectId: ProjectId,
  id: number,
  choice: VersionChoice,
  expected?: string | null,
) {
  const body: Record<string, unknown> = { ...choice };
  if (expected !== undefined) body.expectedPlannedVersion = expected;
  try {
    return await send<{ task: Task; changed: boolean }>(
      'PUT',
      `${projectPath(projectId)}/tasks/${id}/planned-version`,
      body,
    );
  } catch (cause) {
    if (cause instanceof WriteError && cause.code === 'planned_version_changed') {
      const current = (cause.body as { currentPlannedVersion?: unknown } | null)
        ?.currentPlannedVersion;
      throw new PlannedVersionChangedError(
        cause.message,
        typeof current === 'string' ? current : null,
      );
    }
    throw cause;
  }
}

/**
 * Sets the planned version of the tasks of an epic that the person left checked. Done and
 * archived tasks are always skipped. With `dryRun` nothing is written, so the answer previews
 * the change.
 */
export function setEpicPlannedVersion(
  projectId: ProjectId,
  epicId: string,
  input: {
    choice: VersionChoice;
    scope: EpicVersionScope;
    taskIds: readonly number[];
    dryRun?: boolean;
  },
) {
  return send<EpicVersionResult>(
    'POST',
    `${projectPath(projectId)}/epics/${epicId}/planned-version`,
    {
      ...input.choice,
      scope: input.scope,
      taskIds: input.taskIds,
      dryRun: input.dryRun ?? false,
    },
  );
}
