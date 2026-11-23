import { useEffect, useSyncExternalStore } from 'react';
import { loadMode } from './api';
import { parseModeEvent, SPARRING, type ProjectMode } from './model';

/**
 * The modes the page knows, by project. The server holds the saved value; this is a copy that the
 * `mode` event and every read or save refresh, so the settings row and the header indicator agree
 * and a change made by the CLI shows without a reload.
 */
const known = new Map<string, ProjectMode>();
const listeners = new Set<() => void>();

function emit() {
  for (const listener of [...listeners]) listener();
}

export function setProjectMode(projectId: string, mode: ProjectMode) {
  known.set(projectId, mode);
  emit();
}

/** Applies a `mode` event of the board's stream (a `MessageEvent`, or its data). */
export function applyModeEvent(event: unknown) {
  const data = (event as { data?: unknown } | null)?.data;
  const changes = parseModeEvent(data);
  if (changes.length === 0) return;
  for (const [id, mode] of changes) known.set(id, mode);
  emit();
}

/** Reads again every project the page shows, for events missed while the stream was down. */
export function refreshModes() {
  for (const id of [...known.keys()]) {
    loadMode(id).then(
      (mode) => setProjectMode(id, mode),
      () => undefined,
    );
  }
}

export function resetModes() {
  known.clear();
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
    if (!projectId || known.has(projectId)) return undefined;
    let current = true;
    loadMode(projectId).then(
      (loaded) => {
        if (current && !known.has(projectId)) setProjectMode(projectId, loaded);
      },
      () => undefined,
    );
    return () => {
      current = false;
    };
  }, [projectId]);
  return mode ?? null;
}

export { SPARRING };
