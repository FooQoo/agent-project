import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ConnectionSnapshot, RegisteredDevice } from './api';

const api = vi.hoisted(() => ({
  getConnection: vi.fn(),
  getDevices: vi.fn(),
  saveSettings: vi.fn(),
  startConnection: vi.fn(),
}));

vi.mock('./api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./api')>()),
  ...api,
}));

import { useConnectionController } from './useConnectionController';

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

const snapshot = (over: Partial<ConnectionSnapshot> = {}): ConnectionSnapshot => ({
  status: 'disabled',
  url: null,
  revision: 1,
  settings: { enabled: false, domain: null },
  domainEnv: null,
  tokenConfigured: true,
  tokenSource: 'keychain',
  keychainAvailable: true,
  keychainTokenSaved: true,
  failure: null,
  ...over,
});

const device: RegisteredDevice = {
  id: 'd1',
  label: 'iPhone / Safari',
  createdAt: '2026-01-01T00:00:00Z',
  lastSeenAt: '2026-01-01T00:00:00Z',
};

describe('useConnectionController', () => {
  it('applies the status and connects while the device list never arrives', async () => {
    api.getConnection.mockResolvedValue(snapshot());
    api.getDevices.mockReturnValue(new Promise(() => {}));
    const next = snapshot({ status: 'ready', revision: 2 });
    api.saveSettings.mockResolvedValue(next);
    const { result } = renderHook(() => useConnectionController(true));
    await waitFor(() => expect(result.current.ready).toBe(true));
    expect(result.current.devices).toEqual([]);
    await act(async () => {
      await result.current.connect({ token: '', save: false, domain: '' });
    });
    expect(api.saveSettings).toHaveBeenCalled();
    expect(result.current.error).toBeNull();
  });

  it('keeps the devices it showed when a later device read fails', async () => {
    api.getConnection.mockResolvedValue(snapshot());
    api.getDevices.mockResolvedValueOnce([device]).mockRejectedValue(new Error('down'));
    const { result } = renderHook(() => useConnectionController(true));
    await waitFor(() => expect(result.current.devices).toEqual([device]));
    await waitFor(() => expect(api.getDevices.mock.calls.length).toBeGreaterThan(1), {
      timeout: 4000,
    });
    expect(result.current.ready).toBe(true);
    expect(result.current.devices).toEqual([device]);
  });

  it('stops saving after a failed save once the status is read, though devices never arrive', async () => {
    api.getConnection.mockResolvedValue(snapshot());
    api.getDevices.mockReturnValue(new Promise(() => {}));
    api.saveSettings.mockRejectedValue(new Error('down'));
    const { result } = renderHook(() => useConnectionController(true));
    await waitFor(() => expect(result.current.ready).toBe(true));
    await act(async () => {
      await result.current.connect({ token: '', save: false, domain: '' });
    });
    expect(result.current.saving).toBe(false);
    expect(result.current.error).not.toBeNull();
  });
});
