import { setupWorker } from 'msw/browser';
import type { MockServer } from '../mock-server/server.ts';
import { mockHandlers } from './handlers.ts';

/** Set while a page that the worker does not control yet has reloaded once to be controlled. */
const RELOADED_FLAG = 'agboard.mockWorkerReloaded';

/** What `startMockWorker` did: the page can render, it is reloading, or it cannot run. */
export type MockWorkerState = 'ready' | 'reloading' | 'unsupported';

/** Browsers with no Service Worker, or a page that is not a secure context, cannot run the mock. */
export function supportsMockWorker(nav: { serviceWorker?: unknown }): boolean {
  return nav.serviceWorker !== undefined;
}

/**
 * Whether a page that the worker does not control should reload: a hard reload (Shift+Reload)
 * loads the page without a controller, and the worker controls it from the next load. It reloads
 * once; if the page is still not controlled afterwards, it runs anyway.
 */
export function shouldReload(controlled: boolean, reloadedBefore: boolean): boolean {
  return !controlled && !reloadedBefore;
}

/**
 * Starts the Service Worker that answers `/api/*`, the sample picture and the event stream from
 * `server`. The worker script sits beside the page (`web/mock-browser.ts` serves it in the dev
 * mode and emits it in the demo build), so the scope is the folder of the page and no header is
 * needed. Call it before the first render, so the board's first request already has an answer.
 */
export async function startMockWorker(server: MockServer): Promise<MockWorkerState> {
  if (!supportsMockWorker(navigator)) return 'unsupported';
  await setupWorker(...mockHandlers(server)).start({
    serviceWorker: { url: `${import.meta.env.BASE_URL}mockServiceWorker.js` },
    onUnhandledFrame: 'bypass',
    quiet: true,
  });
  const controlled = navigator.serviceWorker.controller !== null;
  const reloadedBefore = sessionStorage.getItem(RELOADED_FLAG) === '1';
  if (shouldReload(controlled, reloadedBefore)) {
    sessionStorage.setItem(RELOADED_FLAG, '1');
    location.reload();
    return 'reloading';
  }
  if (controlled) sessionStorage.removeItem(RELOADED_FLAG);
  return 'ready';
}
