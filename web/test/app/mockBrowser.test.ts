// The start of the browser mock: when a browser cannot run it, and when a page reloads once to be
// controlled by the worker.
import { describe, expect, it } from 'vitest';
import { shouldReload, supportsMockWorker } from '../../src/app/development/mocks/browser';

describe('supportsMockWorker', () => {
  it('needs a Service Worker container on the navigator', () => {
    expect(supportsMockWorker({ serviceWorker: {} })).toBe(true);
    expect(supportsMockWorker({})).toBe(false);
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
