// The start of the browser mock: a worker that cannot start leaves the board unrendered and says so.
import { beforeEach, describe, expect, it, vi } from 'vitest';

const startMockWorker = vi.fn();
vi.mock('../../src/app/development/mocks/browser', () => ({ startMockWorker }));
vi.mock('../../src/app/development/mock-server/server', () => ({
  createMockServer: () => ({}),
}));

const { startMock } = await import('../../src/app/development/mocks/start');

describe('startMock', () => {
  let root: HTMLElement;
  beforeEach(() => {
    startMockWorker.mockReset();
    root = document.createElement('div');
  });

  it('renders the board when the worker is ready', async () => {
    startMockWorker.mockResolvedValue('ready');
    expect(await startMock('mock', root)).toBe(true);
    expect(root.textContent).toBe('');
  });

  it('shows a notice and no board when the worker fails to start', async () => {
    startMockWorker.mockRejectedValue(new Error('script blocked'));
    expect(await startMock('mock', root)).toBe(false);
    expect(root.querySelector('[role="alert"]')?.textContent).toContain('could not start');
  });

  it('shows the unsupported notice and no board without Service Workers', async () => {
    startMockWorker.mockResolvedValue('unsupported');
    expect(await startMock('mock', root)).toBe(false);
    expect(root.querySelector('[role="alert"]')?.textContent).toContain('needs Service Workers');
  });

  it('does not render while the page reloads to be controlled', async () => {
    startMockWorker.mockResolvedValue('reloading');
    expect(await startMock('mock', root)).toBe(false);
    expect(root.textContent).toBe('');
  });
});
