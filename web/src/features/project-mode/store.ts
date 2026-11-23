import { useEffect, useSyncExternalStore } from 'react';
import { loadMode } from './api';
import { parseModeEvent, SPARRING, type ProjectMode } from './model';

/**
 * The modes the page knows, by project. The server holds the saved value; this is a copy that the
 * `mode` event and every read or save refresh, so the settings row and the header indicator agree
 * and a change made by the CLI shows without a reload.
 */
const known = new Map<string, ProjectMode>();
/** Projects the page shows, whether or not a read has succeeded, so a reconnect retries them. */
const displayed = new Set<string>();
/** Counts the updates of each project, so a read that started earlier does not overwrite them. */
const versions = new Map<string, number>();
/** The latest read started for each project, so a read that a newer read replaced is dropped. */
const reads = new Map<string, number>();
const listeners = new Set<() => void>();

function put(projectId: string, mode: ProjectMode) {
  known.set(projectId, mode);
  versions.set(projectId, (versions.get(projectId) ?? 0) + 1);
}

/** Reads a project's mode and applies it only if nothing newer arrived while the read ran. */
function loadAndApply(projectId: string) {
  const started = versions.get(projectId) ?? 0;
  const read = (reads.get(projectId) ?? 0) + 1;
  reads.set(projectId, read);
  return loadMode(projectId).then(
    (mode) => {
      if (reads.get(projectId) === read && (versions.get(projectId) ?? 0) === started) {
        setProjectMode(projectId, mode);
      }
    },
    () => undefined,
  );
}

function emit() {
  for (const listener of [...listeners]) listener();
}

export function setProjectMode(projectId: string, mode: ProjectMode) {
  put(projectId, mode);
  emit();
}

/** Applies a `mode` event of the board's stream (a `MessageEvent`, or its data). */
export function applyModeEvent(event: unknown) {
  const data = (event as { data?: unknown } | null)?.data;
  const changes = parseModeEvent(data);
  if (changes.length === 0) return;
  for (const [id, mode] of changes) put(id, mode);
  emit();
}

/** Reads again every project the page shows, for events missed while the stream was down. */
export function refreshModes() {
  for (const id of new Set([...displayed, ...known.keys()])) void loadAndApply(id);
}

export function resetModes() {
  known.clear();
  displayed.clear();
  versions.clear();
  reads.clear();
  emit();
}

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
};

/**
 * The project's mode, or `null` while it is not known yet. It is read once when the project is
 * first shown, and kept current by `applyModeEvent`. A project that cannot be read stays unknown,
 * so nothing claims a mode the server did not confirm.
 */
export function useProjectMode(projectId: string | null): ProjectMode | null {
  const mode = useSyncExternalStore(subscribe, () =>
    projectId ? known.get(projectId) : undefined,
  );
  useEffect(() => {
    if (!projectId) return;
    displayed.add(projectId);
    if (!known.has(projectId)) void loadAndApply(projectId);
  }, [projectId]);
  return mode ?? null;
}

export { SPARRING };
