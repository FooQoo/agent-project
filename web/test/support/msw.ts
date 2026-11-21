import type { UnhandledFrameHandle } from 'msw';
import { setupServer } from 'msw/node';
import { mockHandlers, type MockHandlerOptions } from '../../src/app/development/mocks/handlers';
import type { MockServer } from '../../src/app/development/mock-server/server';

/** The MSW server of the unit tests. `test/setup.ts` starts it, and resets it after each test. */
export const mswServer = setupServer();

/** Answers `/api/*` and the event stream from `server` until the end of the test. */
export function useMockServer(server: MockServer, options?: MockHandlerOptions): void {
  mswServer.use(...mockHandlers(server, options));
}

/** Product requests no handler answered during the current test, as `METHOD url`. */
const unhandled: string[] = [];

/**
 * A product request (`/api/` or `/__mock/`) that no handler answers is an error, and it is
 * recorded so the test that made it fails at its end even when the code under test caught the error.
 */
export const failOnUnhandledProductRequest: UnhandledFrameHandle = ({ frame, defaults }) => {
  if (frame.protocol !== 'http') return;
  const { request } = frame.data as { request: Request };
  const { pathname } = new URL(request.url);
  if (!pathname.startsWith('/api/') && !pathname.startsWith('/__mock/')) return;
  unhandled.push(`${request.method} ${request.url}`);
  defaults.error();
};

/** Clears the record and returns what it held. A test that expects such requests calls it. */
export function takeUnhandledProductRequests(): string[] {
  return unhandled.splice(0);
}

/** Throws when a product request went unanswered during the test; called after each test. */
export function assertNoUnhandledProductRequests(): void {
  const requests = takeUnhandledProductRequests();
  if (requests.length > 0)
    throw new Error(`Product requests no handler answered:\n${requests.join('\n')}`);
}
