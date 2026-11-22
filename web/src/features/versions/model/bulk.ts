import type {
  EpicVersionResult,
  EpicVersionScope,
  SkipReason,
  VersionChoice,
} from '../api/versions';

/** The choices of the bulk change's target row: a version, the current one, or none. */
export type BulkTarget =
  | { kind: 'named'; version: string }
  | { kind: 'current' }
  | { kind: 'unset' };

export function choiceOf(target: BulkTarget): VersionChoice {
  switch (target.kind) {
    case 'named':
      return { choice: 'named', version: target.version };
    case 'current':
      return { choice: 'current' };
    case 'unset':
      return { choice: 'unset' };
  }
}

/** The version name the target saves, given the project's current version. */
export function targetName(target: BulkTarget, current: string | null): string | null {
  if (target.kind === 'named') return target.version;
  if (target.kind === 'current') return current;
  return null;
}

/** An epic's task as the dialog lists it. */
export interface BulkMember {
  id: number;
  title: string;
  taskStatus: string;
  plannedVersion: string | null;
}

export type BulkRow =
  | { id: number; title: string; from: string | null; state: 'change' | 'same' }
  | { id: number; title: string; from: string | null; state: 'skipped'; reason: SkipReason };

/** One row for each task: what the server would do with it, from its dry run. */
export function bulkRows(members: readonly BulkMember[], result: EpicVersionResult): BulkRow[] {
  const changed = new Set(result.changed);
  const unchanged = new Set(result.unchanged);
  return members.map((member): BulkRow => {
    const base = { id: member.id, title: member.title, from: member.plannedVersion };
    if (changed.has(member.id)) return { ...base, state: 'change' };
    if (unchanged.has(member.id)) return { ...base, state: 'same' };
    const reason = result.skippedReasons[String(member.id)] ?? 'planned';
    return { ...base, state: 'skipped', reason };
  });
}

/** The IDs to send: the rows that would change and that the person left checked. */
export function checkedIds(rows: readonly BulkRow[], unchecked: ReadonlySet<number>): number[] {
  return rows.filter((row) => row.state === 'change' && !unchecked.has(row.id)).map((r) => r.id);
}

export type { EpicVersionScope };
