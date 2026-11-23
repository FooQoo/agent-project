import { authHeaders } from '../../shared/api/http';
import { parseProjectMode, type ModeKind, type ProjectMode } from './model';

const url = (projectId: string) => `/api/projects/${projectId}/mode`;

/** A failed read or save. `status` is 0 when the request never got an answer. */
export class ModeError extends Error {
  constructor(readonly status: number) {
    super(`mode request failed (${status})`);
  }
}

async function parse(response: Response): Promise<ProjectMode> {
  if (!response.ok) throw new ModeError(response.status);
  const mode = parseProjectMode(await response.json().catch(() => null));
  if (!mode) throw new ModeError(response.status);
  return mode;
}

/** The saved mode. Served on the PC and through the remote entrance. */
export async function loadMode(projectId: string, fetcher: typeof fetch = fetch) {
  let response: Response;
  try {
    response = await fetcher(url(projectId), {
      cache: 'no-store',
      headers: { Accept: 'application/json' },
    });
  } catch {
    throw new ModeError(0);
  }
  return parse(response);
}

/** Saves the mode and returns the saved value. */
export async function saveMode(projectId: string, mode: ModeKind, fetcher: typeof fetch = fetch) {
  let response: Response;
  try {
    response = await fetcher(url(projectId), {
      method: 'PUT',
      cache: 'no-store',
      headers: {
        Accept: 'application/json',
        'Content-Type': 'application/json',
        ...authHeaders(),
      },
      body: JSON.stringify({ mode }),
    });
  } catch {
    throw new ModeError(0);
  }
  return parse(response);
}
