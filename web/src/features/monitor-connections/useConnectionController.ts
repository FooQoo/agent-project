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
import { useWrite } from '../../shared/data/react/use-write';
import { qrDataUrl } from './qr';
import { isProjectIdValue, phoneUrl } from './phoneUrl';

const POLL_MS = 1500;

/** What a write of the dialog does; one at a time. */
type ConnectionWrite =
  | { kind: 'connect'; draft: ConnectionDraft; token: string; snapshot: ConnectionSnapshot }
  | { kind: 'stop' }
  | { kind: 'token' }
  | { kind: 'device'; id: string };

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
  const [error, setError] = useState<string | null>(null);

  // The status and the devices are two reads: a slow or failed device list never holds back the
  // status. When the list cannot be read the dialog keeps what it showed.
  const query = useReadQuery({
    queryKey: queryKeys.connection(),
    queryFn: getConnection,
    enabled: open || transientRef.current,
    refetchIntervalMs: POLL_MS,
  });
  const devicesQuery = useReadQuery({
    queryKey: queryKeys.connectionDevices(),
    queryFn: getDevices,
    enabled: open || transientRef.current,
    refetchIntervalMs: POLL_MS,
  });
  const { refetch, data: read, dataUpdatedAt } = query;
  const {
    refetch: refetchDevices,
    data: deviceList,
    dataUpdatedAt: devicesUpdatedAt,
  } = devicesQuery;
  const loadFailed = query.isError;

  // biome-ignore lint/correctness/useExhaustiveDependencies: every finished read applies, changed or not.
  useEffect(() => {
    if (read && !write.isPending) setSnapshot(read);
  }, [dataUpdatedAt]);
  // biome-ignore lint/correctness/useExhaustiveDependencies: every finished read applies, changed or not.
  useEffect(() => {
    if (Array.isArray(deviceList) && !write.isPending) setDevices(deviceList);
  }, [devicesUpdatedAt]);
  // Only the status is awaited: a stalled device read must never keep a save pending. The device
  // list is refreshed on its own and applies when it arrives.
  const refresh = useCallback(async () => {
    void refetchDevices({ cancelRefetch: false }).then((read) => {
      if (Array.isArray(read.data)) setDevices(read.data);
    });
    const result = await refetch({ cancelRefetch: false });
    if (result.data) setSnapshot(result.data);
  }, [refetch, refetchDevices]);

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
          case 'invalid_provider':
            return t('monitorConnectionPc.error.invalid');
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

  // One write at a time: a second submit while one is on its way is dropped. A failure says why;
  // after a failed save only the status is read again (the device list is never awaited).
  const write = useWrite<void, ConnectionWrite>({
    mutationKey: ['connection-write'],
    mutationFn: async (op) => {
      if (op.kind === 'stop') setSnapshot(await stopConnection());
      else if (op.kind === 'token') setSnapshot(await deleteSavedToken());
      else if (op.kind === 'device') setDevices(await deleteDevice(op.id));
      else {
        const { draft, token, snapshot: base } = op;
        let next = await saveSettings({
          enabled: true,
          revision: base.revision,
          provider: draft.provider,
          // The domain is ngrok's. The environment variable decides when it is set, so the
          // field is not sent then.
          ...(base.domainEnv || draft.provider === 'tailscale'
            ? {}
            : { domain: draft.domain.trim() }),
        });
        setSnapshot(next);
        const running =
          next.status === 'starting' || next.status === 'ready' || next.status === 'reconnecting';
        // A new token always restarts; otherwise a saved change has already restarted a live tunnel.
        if (token || !running)
          next = await startConnection(
            token || null,
            Boolean(token) && draft.save && base.keychainAvailable,
          );
        setSnapshot(next);
      }
      if (op.kind === 'stop' || op.kind === 'connect') setEditing(false);
    },
    onMutate: () => setError(null),
    onError: async (cause, op) => {
      setError(messageOf(cause));
      // The server may hold newer settings after a conflict; show what is really saved.
      if (op.kind === 'connect') await refresh();
      // The device may already be gone; show what is really registered.
      else if (op.kind === 'device')
        try {
          setDevices(await getDevices());
        } catch {
          // Left as it was.
        }
    },
  });
  const { mutate, isWriting } = write;
  const saving = write.isPending;

  const connect = useCallback(
    (draft: ConnectionDraft) => {
      if (isWriting()) return;
      if (!snapshot) {
        setError(t('monitorConnectionPc.error.unreachable'));
        return;
      }
      // Tailscale needs no token and the field is not shown, so none is sent.
      const token = draft.provider === 'tailscale' ? '' : draft.token.trim();
      // An environment or saved token is enough; the server answers token_required otherwise.
      if (draft.provider === 'ngrok' && !token && !snapshot.tokenConfigured) {
        setError(t('monitorConnectionPc.error.tokenRequired'));
        return;
      }
      mutate({ kind: 'connect', draft, token, snapshot });
    },
    [snapshot, t, isWriting, mutate],
  );
  const stop = useCallback(() => {
    if (!isWriting()) mutate({ kind: 'stop' });
  }, [isWriting, mutate]);
  const removeToken = useCallback(() => {
    if (!isWriting()) mutate({ kind: 'token' });
  }, [isWriting, mutate]);
  const removeRegisteredDevice = useCallback(
    (id: string) => {
      if (!isWriting()) mutate({ kind: 'device', id });
    },
    [isWriting, mutate],
  );

  const savedDomain = snapshot?.settings.domain ?? '';
  const savedProvider = snapshot?.settings.provider ?? 'ngrok';
  const draft = useMemo<ConnectionDraft>(
    () => ({ provider: savedProvider, token: '', save: false, domain: savedDomain }),
    [savedProvider, savedDomain],
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
