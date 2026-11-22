import type { Task } from '../../../shared/api/contracts.ts';
import type { Routes } from './types.ts';
import {
  error,
  forbidden,
  invalidJson,
  json,
  parseBody,
  projectNotFound,
  taskChange,
  toWireTask,
} from './wire.ts';

// The versions of a project and the planned version of its tasks, as `agboard serve` answers them.

const BASE = /^\/api\/projects\/([^/]+)\/(versions|versions\/current)$/;
const TASK = /^\/api\/projects\/([^/]+)\/tasks\/(\d+)\/planned-version$/;
const EPIC = /^\/api\/projects\/([^/]+)\/epics\/([^/]+)\/planned-version$/;

/** The server lists versions newest first, comparing digit runs as numbers. */
const newestFirst = (versions: string[]) =>
  [...versions].sort((a, b) => b.localeCompare(a, 'en', { numeric: true }));

const invalid = (reason: string) => error(422, 'validation', 'invalid version.', reason);

type Choice = { choice: 'named'; version: string } | { choice: 'current' } | { choice: 'unset' };

/** The version a choice names for this project; `undefined` when the choice cannot be applied. */
function resolve(
  input: Record<string, unknown>,
  versions: string[],
  current: string | null,
): { version: string | null } | { problem: string } {
  const choice = input.choice as Choice['choice'] | undefined;
  if (choice === 'unset') return { version: null };
  if (choice === 'current')
    return current === null
      ? { problem: 'The project has no current version.' }
      : { version: current };
  if (choice === 'named') {
    const version = input.version;
    if (typeof version !== 'string' || !versions.includes(version))
      return { problem: 'The version is not listed in the project.' };
    return { version };
  }
  return { problem: 'Send choice as named, current or unset.' };
}

export const versionRoutes: Routes = (context, request) => {
  const { method } = request;
  const path = request.url.pathname;
  const base = BASE.exec(path);
  const task = TASK.exec(path);
  const epic = EPIC.exec(path);
  const projectId = (base ?? task ?? epic)?.[1];
  if (projectId === undefined) return null;
  const found = context.boards()[projectId];

  if (base && method === 'GET') {
    if (!found) return projectNotFound();
    return json(200, {
      versions: newestFirst(found.project.versions ?? []),
      currentVersion: found.project.currentVersion ?? null,
    });
  }
  const writes =
    (base && (method === 'POST' || method === 'PUT')) ||
    (task && method === 'PUT') ||
    (epic && method === 'POST');
  if (!writes) return null;
  if (!request.token) return forbidden();
  if (!found) return projectNotFound();
  const input = parseBody(request.body);
  if (!input) return invalidJson();
  const versions = found.project.versions ?? [];
  const current = found.project.currentVersion ?? null;
  const settings = (next: { versions?: string[]; currentVersion?: string | null }) =>
    context.commit(projectId, { ...found, project: { ...found.project, ...next } }, [
      { resource: 'project', id: projectId, facets: ['summary'] },
    ]);

  if (base?.[2] === 'versions' && method === 'POST') {
    const name = typeof input.name === 'string' ? input.name.trim() : '';
    if (name === '') return invalid('Send the version name as name.');
    const changed = !versions.includes(name);
    if (changed) settings({ versions: [...versions, name] });
    return json(200, {
      versions: newestFirst(changed ? [...versions, name] : versions),
      currentVersion: current,
      changed,
    });
  }
  if (base) {
    // PUT /versions/current
    const version = input.version;
    if (version !== null && typeof version !== 'string')
      return invalid('Send version as a name or null.');
    const listed =
      version === null || versions.includes(version) ? versions : [...versions, version];
    const changed = version !== current;
    if (changed || listed !== versions) settings({ versions: listed, currentVersion: version });
    return json(200, { versions: newestFirst(listed), currentVersion: version, changed });
  }

  if (task) {
    const id = Number(task[2]);
    const item = found.tasks.find((entry) => entry.id === id);
    if (!item)
      return error(404, 'task_not_found', `task #${id} not found.`, `No task has ID ${id}.`);
    const resolved = resolve(input, versions, current);
    if ('problem' in resolved) return invalid(resolved.problem);
    const now = item.plannedVersion ?? null;
    if ('expectedPlannedVersion' in input && (input.expectedPlannedVersion ?? null) !== now)
      return json(409, {
        error: {
          code: 'planned_version_changed',
          message: `the planned version of task #${id} changed.`,
          reason: 'Read the task again before changing it.',
        },
        currentPlannedVersion: now,
      });
    if (resolved.version === now) return json(200, { task: toWireTask(item), changed: false });
    const updated: Task = { ...item, plannedVersion: resolved.version, updatedAt: context.now() };
    context.replaceTasks(found, [updated], [taskChange(id, ['summary', 'document'])]);
    return json(200, { task: toWireTask(updated), changed: true });
  }

  // POST /epics/{id}/planned-version
  const epicId = decodeURIComponent(epic?.[2] ?? '');
  const resolved = resolve(input, versions, current);
  if ('problem' in resolved) return invalid(resolved.problem);
  const scope = input.scope === 'all' ? 'all' : 'unplanned-only';
  const ids = Array.isArray(input.taskIds)
    ? input.taskIds.filter((n): n is number => typeof n === 'number')
    : [];
  const result = {
    changed: [] as number[],
    unchanged: [] as number[],
    skipped: [] as number[],
    skippedReasons: {} as Record<string, string>,
    dryRun: input.dryRun === true,
  };
  const updates: Task[] = [];
  for (const id of ids) {
    const item = found.tasks.find((entry) => entry.id === id && entry.epic?.id === epicId);
    if (!item) continue;
    const reason = item.archivedAt
      ? 'archived'
      : item.taskStatus === 'done'
        ? 'done'
        : scope === 'unplanned-only' && item.plannedVersion
          ? 'planned'
          : null;
    if (reason !== null) {
      result.skipped.push(id);
      result.skippedReasons[String(id)] = reason;
    } else if ((item.plannedVersion ?? null) === resolved.version) result.unchanged.push(id);
    else {
      result.changed.push(id);
      updates.push({ ...item, plannedVersion: resolved.version, updatedAt: context.now() });
    }
  }
  if (!result.dryRun && updates.length > 0)
    context.replaceTasks(
      found,
      updates,
      updates.map((entry) => taskChange(entry.id, ['summary', 'document'])),
    );
  return json(200, result);
};
