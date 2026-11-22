import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ConnectionRequestError, type ConnectionSnapshot, type RegisteredDevice } from './api';

const api = vi.hoisted(() => ({
  getConnection: vi.fn(),
  getDevices: vi.fn(),
  saveSettings: vi.fn(),
  startConnection: vi.fn(),
  stopConnection: vi.fn(),
  removeDevice: vi.fn(),
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
  settings: { enabled: false, provider: 'ngrok', domain: null },
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

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

describe('useConnectionController', () => {
  it('applies the status and connects while the device list never arrives', async () => {
    api.getConnection.mockResolvedValue(snapshot());
    api.getDevices.mockReturnValue(new Promise(() => {}));
    const next = snapshot({ status: 'ready', revision: 2 });
    api.saveSettings.mockResolvedValue(next);
    const { result } = renderHook(() => useConnectionController(true));
    await waitFor(() => expect(result.current.ready).toBe(true));
    expect(result.current.devices).toEqual([]);
    act(() => result.current.connect({ provider: 'ngrok', token: '', save: false, domain: '' }));
    await waitFor(() => expect(api.saveSettings).toHaveBeenCalled());
    await waitFor(() => expect(result.current.saving).toBe(false));
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
    act(() => result.current.connect({ provider: 'ngrok', token: '', save: false, domain: '' }));
    await waitFor(() => expect(result.current.saving).toBe(false));
    expect(api.saveSettings).toHaveBeenCalled();
    expect(result.current.error).not.toBeNull();
  });

  it('drops a second submit while a save is on its way', async () => {
    api.getConnection.mockResolvedValue(snapshot());
    api.getDevices.mockResolvedValue([]);
    const gate = deferred<ConnectionSnapshot>();
    api.saveSettings.mockReturnValue(gate.promise);
    api.startConnection.mockResolvedValue(snapshot({ status: 'ready' }));
    const { result } = renderHook(() => useConnectionController(true));
    await waitFor(() => expect(result.current.ready).toBe(true));
    act(() => {
      result.current.connect({ provider: 'ngrok', token: '', save: false, domain: '' });
      result.current.connect({ provider: 'ngrok', token: '', save: false, domain: '' });
    });
    await waitFor(() => expect(result.current.saving).toBe(true));
    gate.resolve(snapshot({ status: 'ready', revision: 2 }));
    await waitFor(() => expect(result.current.saving).toBe(false));
    expect(api.saveSettings).toHaveBeenCalledTimes(1);
  });

  it('shows the fixed message of a refused stop and applies the stopped status on success', async () => {
    api.getConnection.mockResolvedValue(snapshot({ status: 'ready' }));
    api.getDevices.mockResolvedValue([]);
    api.stopConnection
      .mockRejectedValueOnce(new ConnectionRequestError(500, 'start_failed'))
      .mockResolvedValueOnce(snapshot({ status: 'disabled' }));
    const { result } = renderHook(() => useConnectionController(true));
    await waitFor(() => expect(result.current.status).toBe('ready'));
    act(() => result.current.stop());
    await waitFor(() => expect(result.current.error).not.toBeNull());
    expect(result.current.saving).toBe(false);
    act(() => result.current.stop());
    await waitFor(() => expect(result.current.status).toBe('disabled'));
    expect(result.current.error).toBeNull();
  });

  it('reads the devices again after a failed removal', async () => {
    api.getConnection.mockResolvedValue(snapshot());
    api.getDevices.mockResolvedValue([device]);
    api.removeDevice.mockRejectedValue(new ConnectionRequestError(404, 'device_not_found'));
    const { result } = renderHook(() => useConnectionController(true));
    await waitFor(() => expect(result.current.devices).toEqual([device]));
    api.getDevices.mockResolvedValue([]);
    act(() => result.current.removeDevice('d1'));
    await waitFor(() => expect(result.current.devices).toEqual([]));
    expect(result.current.error).not.toBeNull();
  });
  it('connects with Tailscale without a token or a domain', async () => {
    api.getConnection.mockResolvedValue(
      snapshot({ tokenConfigured: false, keychainTokenSaved: false, tokenSource: null }),
    );
    api.getDevices.mockResolvedValue([]);
    api.saveSettings.mockResolvedValue(
      snapshot({
        revision: 2,
        settings: { enabled: true, provider: 'tailscale', domain: null },
      }),
    );
    api.startConnection.mockResolvedValue(snapshot({ status: 'starting', revision: 2 }));
    const { result } = renderHook(() => useConnectionController(true));
    await waitFor(() => expect(result.current.ready).toBe(true));
    act(() =>
      result.current.connect({
        provider: 'tailscale',
        token: 'left-over',
        save: true,
        domain: 'old.ngrok-free.dev',
      }),
    );
    await waitFor(() => expect(api.startConnection).toHaveBeenCalled());
    expect(api.saveSettings).toHaveBeenCalledWith({
      enabled: true,
      revision: 1,
      provider: 'tailscale',
    });
    expect(api.startConnection).toHaveBeenCalledWith(null, false);
    expect(result.current.error).toBeNull();
  });

  it('still asks for the token with ngrok when none is held', async () => {
    api.getConnection.mockResolvedValue(
      snapshot({ tokenConfigured: false, keychainTokenSaved: false, tokenSource: null }),
    );
    api.getDevices.mockResolvedValue([]);
    const { result } = renderHook(() => useConnectionController(true));
    await waitFor(() => expect(result.current.ready).toBe(true));
    act(() => result.current.connect({ provider: 'ngrok', token: '', save: false, domain: '' }));
    await waitFor(() => expect(result.current.error).not.toBeNull());
    expect(api.saveSettings).not.toHaveBeenCalled();
  });
});
