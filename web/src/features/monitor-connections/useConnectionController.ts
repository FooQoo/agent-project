import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ConnectionRequestError,
  deleteSavedToken,
  getConnection,
  getDevices,
  removeDevice as deleteDevice,
  saveSettings,
  startConnection,
  stopConnection,
  type ConnectionSnapshot,
  type RegisteredDevice,
} from './api';
import type {
  ConnectionDraft,
  ConnectionQr,
  ConnectionStatus,
  TokenInfo,
} from './ui/connectionTypes';
import { queryKeys } from '../../shared/data/core/query-keys';
import { useReadQuery } from '../../shared/data/react';
import { qrDataUrl } from './qr';
import { isProjectIdValue, phoneUrl } from './phoneUrl';

const POLL_MS = 1500;

interface ConnectionRead {
  snapshot: ConnectionSnapshot;
  /** Null when the list could not be read. */
  devices: RegisteredDevice[] | null;
}

async function readConnection(): Promise<ConnectionRead> {
  const snapshot = await getConnection();
  try {
    const list = await getDevices();
    return { snapshot, devices: Array.isArray(list) ? list : null };
  } catch {
    return { snapshot, devices: null };
  }
}

/**
 * The state behind the connection dialog: it reads the status from the local API, saves the
 * settings, starts and stops. The token goes straight to the start request and is kept nowhere.
 */
export function useConnectionController(open: boolean, projectId: string | null = null) {
  const { t } = useTranslation();
  const [snapshot, setSnapshot] = useState<ConnectionSnapshot | null>(null);
  const [devices, setDevices] = useState<RegisteredDevice[]>([]);
  const transientRef = useRef(false);
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const savingRef = useRef(false);

  // The status and the devices are one read. The list is secondary: when it cannot be read the
  // dialog keeps what it showed.
  const query = useReadQuery({
    queryKey: queryKeys.connection(),
    queryFn: readConnection,
    enabled: open || transientRef.current,
    refetchIntervalMs: POLL_MS,
  });
  const { refetch, data: read, dataUpdatedAt } = query;
  const loadFailed = query.isError;

  const apply = useCallback((result: ConnectionRead) => {
    if (savingRef.current) return;
    setSnapshot(result.snapshot);
    if (result.devices) setDevices(result.devices);
  }, []);
  // biome-ignore lint/correctness/useExhaustiveDependencies: every finished read applies, changed or not.
  useEffect(() => {
    if (read) apply(read);
  }, [dataUpdatedAt, apply]);
  const refresh = useCallback(async () => {
    const result = await refetch({ cancelRefetch: false });
    if (result.data) apply(result.data);
  }, [refetch, apply]);

  const status: ConnectionStatus | null = snapshot?.status ?? null;
  transientRef.current =
    status === 'starting' || status === 'reconnecting' || status === 'stopping';

  const messageOf = useCallback(
    (cause: unknown): string => {
      if (cause instanceof ConnectionRequestError) {
        switch (cause.code) {
          case 'revision_conflict':
            return t('monitorConnectionPc.error.conflict');
          case 'save_failed':
            return t('monitorConnectionPc.error.saveFailed');
          case 'token_required':
            return t('monitorConnectionPc.error.tokenRequired');
          case 'invalid_domain':
            return t('monitorConnectionPc.error.invalidDomain');
          case 'invalid_value':
            return t('monitorConnectionPc.error.invalid');
          case 'settings_unreadable':
            return t('monitorConnectionPc.error.unreadable');
          case 'keychain_unavailable':
            return t('monitorConnectionPc.error.keychainUnavailable');
          case 'start_failed':
            return t('monitorConnectionPc.error.startFailed');
          case 'device_not_found':
            return t('monitorConnectionPc.error.deviceNotFound');
          default:
            return t('monitorConnectionPc.error.unreachable');
        }
      }
      return t('monitorConnectionPc.error.unreachable');
    },
    [t],
  );

  const connect = useCallback(
    async (draft: ConnectionDraft) => {
      if (savingRef.current) return;
      if (!snapshot) {
        setError(t('monitorConnectionPc.error.unreachable'));
        return;
      }
      const token = draft.token.trim();
      // An environment or saved token is enough; the server answers token_required otherwise.
      if (!token && !snapshot.tokenConfigured) {
        setError(t('monitorConnectionPc.error.tokenRequired'));
        return;
      }
      savingRef.current = true;
      setSaving(true);
      setError(null);
      try {
        let next = await saveSettings({
          enabled: true,
          revision: snapshot.revision,
          // The environment variable decides when it is set, so the field is not sent then.
          ...(snapshot.domainEnv ? {} : { domain: draft.domain.trim() }),
        });
        setSnapshot(next);
        const running =
          next.status === 'starting' || next.status === 'ready' || next.status === 'reconnecting';
        // A new token always restarts; otherwise a saved change has already restarted a live tunnel.
        if (token || !running)
          next = await startConnection(
            token || null,
            Boolean(token) && draft.save && snapshot.keychainAvailable,
          );
        setSnapshot(next);
        setEditing(false);
      } catch (cause) {
        setError(messageOf(cause));
        // The server may hold newer settings after a conflict; show what is really saved.
        savingRef.current = false;
        await refresh();
      } finally {
        savingRef.current = false;
        setSaving(false);
      }
    },
    [snapshot, t, messageOf, refresh],
  );

  const stop = useCallback(async () => {
    if (savingRef.current) return;
    savingRef.current = true;
    setSaving(true);
    setError(null);
    try {
      setSnapshot(await stopConnection());
      setEditing(false);
    } catch (cause) {
      setError(messageOf(cause));
    } finally {
      savingRef.current = false;
      setSaving(false);
    }
  }, [messageOf]);

  const removeToken = useCallback(async () => {
    if (savingRef.current) return;
    savingRef.current = true;
    setSaving(true);
    setError(null);
    try {
      setSnapshot(await deleteSavedToken());
    } catch (cause) {
      setError(messageOf(cause));
    } finally {
      savingRef.current = false;
      setSaving(false);
    }
  }, [messageOf]);

  const removeRegisteredDevice = useCallback(
    async (id: string) => {
      if (savingRef.current) return;
      savingRef.current = true;
      setSaving(true);
      setError(null);
      try {
        setDevices(await deleteDevice(id));
      } catch (cause) {
        setError(messageOf(cause));
        // The device may already be gone; show what is really registered.
        try {
          setDevices(await getDevices());
        } catch {
          // Left as it was.
        }
      } finally {
        savingRef.current = false;
        setSaving(false);
      }
    },
    [messageOf],
  );

  const savedDomain = snapshot?.settings.domain ?? '';
  const draft = useMemo<ConnectionDraft>(
    () => ({ token: '', save: false, domain: savedDomain }),
    [savedDomain],
  );
  const tokenInfo = useMemo<TokenInfo>(
    () => ({
      source: snapshot?.tokenSource ?? null,
      keychainAvailable: snapshot?.keychainAvailable ?? true,
      saved: snapshot?.keychainTokenSaved ?? false,
    }),
    [snapshot],
  );
  // The QR code and the copied URL are the same address, and both need the selected project.
  const url = useMemo(
    () => (snapshot?.status === 'ready' ? phoneUrl(snapshot.url, projectId) : null),
    [snapshot, projectId],
  );
  // The server sends the passcode only while the connection is ready; show nothing otherwise.
  const passcode = snapshot?.status === 'ready' ? (snapshot.passcode ?? null) : null;
  const qr: ConnectionQr | null = useMemo(() => (url ? { src: qrDataUrl(url) } : null), [url]);

  // The failure reason is a fixed code, never a message from the network.
  const failure =
    snapshot?.status === 'failed' && snapshot.failure
      ? t(`monitorConnectionPc.failure.${snapshot.failure}`)
      : null;

  // Changing the token of a live or stopped connection shows the setup again; the live or stopped
  // view returns after Save and connect, or when the dialog is closed.
  const shown: ConnectionStatus =
    editing && (status === 'ready' || status === 'disabled')
      ? 'needs_config'
      : (status ?? 'disabled');

  return {
    ready: snapshot !== null,
    loadFailed,
    status: shown,
    draft,
    tokenInfo,
    domainEnv: snapshot?.domainEnv ?? null,
    removeToken,
    devices,
    removeDevice: removeRegisteredDevice,
    url,
    projectSelected: isProjectIdValue(projectId),
    passcode,
    qr,
    error: error ?? failure ?? (loadFailed ? t('monitorConnectionPc.error.unreachable') : null),
    saving,
    connect,
    stop,
    resetToken: () => setEditing(true),
    cancelEdit: () => setEditing(false),
  };
}
