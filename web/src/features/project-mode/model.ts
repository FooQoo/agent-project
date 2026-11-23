/**
 * How the agent works with the user in a project. Sparring is the default. Brainstorm asks about
 * one epic or task, so it needs a target and is started from that card or epic, not chosen here.
 */
export type ModeKind = 'sparring' | 'sleep' | 'brainstorm';

/** The modes the settings switch offers: brainstorm is started from its target. */
export type ChoosableMode = 'sparring' | 'sleep';

export const MODE_KINDS: readonly ChoosableMode[] = ['sparring', 'sleep'];

/** The epic or the task a brainstorm is about; a task's `id` is its number as text. */
export interface BrainstormTarget {
  type: 'epic' | 'task';
  id: string;
}

/** How a target is named: `#680` for a task, the ID for an epic. */
export function targetLabel(target: BrainstormTarget): string {
  return target.type === 'task' ? `#${target.id}` : target.id;
}

export const sameTarget = (a: BrainstormTarget | null, b: BrainstormTarget): boolean =>
  a !== null && a.type === b.type && a.id === b.id;

export function parseTarget(value: unknown): BrainstormTarget | null {
  if (typeof value !== 'object' || value === null) return null;
  const { type, id } = value as Record<string, unknown>;
  if ((type !== 'epic' && type !== 'task') || typeof id !== 'string' || id === '') return null;
  return { type, id };
}

/** The saved mode of a project, as `GET`/`PUT /api/projects/{id}/mode` and the `mode` event send it. */
export interface ProjectMode {
  /** The mode in effect now. */
  mode: ModeKind;
  /** Where it comes from: set by hand, or decided by the daily window. */
  modeSource: ModeSource;
  /** When a manual mode was set. */
  since: string | null;
  /** The end of the daily window, while the schedule has the project asleep. */
  until: string | null;
  /** The mode set by hand, if any; a schedule that is on is overridden while it is set. */
  manual: ModeKind | null;
  /** The daily sleep window, in the local time of the machine that runs the server. */
  schedule: Schedule | null;
  /** The epic or task being brainstormed; set exactly while `mode` is `brainstorm`. */
  target: BrainstormTarget | null;
}

export type ModeSource = 'manual' | 'schedule';

/** `start` and `end` are `HH:MM`; the window may cross midnight. */
export interface Schedule {
  enabled: boolean;
  start: string;
  end: string;
}

export const DEFAULT_SCHEDULE: Schedule = { enabled: false, start: '23:00', end: '08:00' };

export const SPARRING: ProjectMode = {
  mode: 'sparring',
  modeSource: 'manual',
  since: null,
  until: null,
  manual: null,
  schedule: null,
  target: null,
};

const TIME = /^\d{2}:\d{2}$/;

function parseSchedule(value: unknown): Schedule | null {
  if (typeof value !== 'object' || value === null) return null;
  const { enabled, start, end } = value as Record<string, unknown>;
  if (typeof enabled !== 'boolean') return null;
  if (typeof start !== 'string' || !TIME.test(start)) return null;
  if (typeof end !== 'string' || !TIME.test(end)) return null;
  return { enabled, start, end };
}

export function isModeKind(value: unknown): value is ModeKind {
  return value === 'sparring' || value === 'sleep' || value === 'brainstorm';
}

/** A mode the settings switch offers (and the API accepts without a target). */
export function isChoosableMode(value: unknown): value is ChoosableMode {
  return typeof value === 'string' && (MODE_KINDS as readonly string[]).includes(value);
}

/** Checks the shape the server sends. Anything else is null, never a partial value. */
export function parseProjectMode(value: unknown): ProjectMode | null {
  if (typeof value !== 'object' || value === null) return null;
  const { mode, modeSource, since, until, manual, schedule, target } = value as Record<
    string,
    unknown
  >;
  if (!isModeKind(mode)) return null;
  const parsedTarget = parseTarget(target);
  // A brainstorm without a target is not one the page can show.
  if (mode === 'brainstorm' && parsedTarget === null) return null;
  return {
    mode,
    modeSource: modeSource === 'schedule' ? 'schedule' : 'manual',
    since: typeof since === 'string' ? since : null,
    until: typeof until === 'string' ? until : null,
    manual: isModeKind(manual) ? manual : null,
    schedule: parseSchedule(schedule),
    target: mode === 'brainstorm' ? parsedTarget : null,
  };
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
