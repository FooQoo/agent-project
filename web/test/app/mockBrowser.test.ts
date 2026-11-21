// The start of the browser mock: when a browser cannot run it, and when a page reloads once to be
// controlled by the worker.
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  shouldReload,
  supportsMockWorker,
  supportsStreamTransfer,
} from '../../src/app/development/mocks/browser';

describe('supportsMockWorker', () => {
  it('starts when there is a Service Worker container and streams can be transferred', () => {
    expect(supportsMockWorker({ serviceWorker: {} }, true)).toBe(true);
  });

  it('shows the notice without a Service Worker container', () => {
    expect(supportsMockWorker({}, true)).toBe(false);
  });

  it('shows the notice when the browser cannot transfer streams', () => {
    expect(supportsMockWorker({ serviceWorker: {} }, false)).toBe(false);
  });
});

describe('supportsStreamTransfer', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('is true when posting a stream to a port works', () => {
    expect(supportsStreamTransfer()).toBe(true);
  });

  it('is false when posting a stream throws', () => {
    vi.stubGlobal(
      'MessageChannel',
      class {
        port1 = {
          postMessage() {
            throw new DOMException('not cloneable', 'DataCloneError');
          },
        };
      },
    );
    expect(supportsStreamTransfer()).toBe(false);
  });
});

describe('shouldReload', () => {
  it('reloads an uncontrolled page once', () => {
    expect(shouldReload(false, false)).toBe(true);
    expect(shouldReload(false, true)).toBe(false);
  });

  it('leaves a controlled page alone', () => {
    expect(shouldReload(true, false)).toBe(false);
    expect(shouldReload(true, true)).toBe(false);
  });
});
