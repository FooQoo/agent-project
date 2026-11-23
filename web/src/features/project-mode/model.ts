/** How the agent works with the user in a project. Sparring is the default. */
export type ModeKind = 'sparring' | 'sleep';

export const MODE_KINDS: readonly ModeKind[] = ['sparring', 'sleep'];

/** The saved mode of a project, as `GET`/`PUT /api/projects/{id}/mode` and the `mode` event send it. */
export interface ProjectMode {
  mode: ModeKind;
  /** When the mode was set; absent for the default that was never set. */
  since: string | null;
}

export const SPARRING: ProjectMode = { mode: 'sparring', since: null };

export function isModeKind(value: unknown): value is ModeKind {
  return typeof value === 'string' && (MODE_KINDS as readonly string[]).includes(value);
}

/** Checks the shape the server sends. Anything else is null, never a partial value. */
export function parseProjectMode(value: unknown): ProjectMode | null {
  if (typeof value !== 'object' || value === null) return null;
  const { mode, since } = value as Record<string, unknown>;
  if (!isModeKind(mode)) return null;
  return { mode, since: typeof since === 'string' ? since : null };
}

/** Reads a `mode` event's data: the projects whose mode changed. Anything else is an empty list. */
export function parseModeEvent(data: unknown): [string, ProjectMode][] {
  let value: unknown = data;
  if (typeof data === 'string') {
    try {
      value = JSON.parse(data);
    } catch {
      return [];
    }
  }
  const projects = (value as { projects?: unknown } | null)?.projects;
  if (typeof projects !== 'object' || projects === null) return [];
  const changes: [string, ProjectMode][] = [];
  for (const [id, entry] of Object.entries(projects)) {
    const mode = parseProjectMode(entry);
    if (mode) changes.push([id, mode]);
  }
  return changes;
}
