import { useCallback, useEffect, useState } from 'react';
import { useWrite } from '../../shared/data/react/use-write';
import { getPushStatus } from './api';
import {
  pushView,
  readSupport,
  type PushPermission,
  type PushStatus,
  type PushSupport,
  type PushView,
} from './state';
import { disablePush, enablePush, hasBrowserSubscription, type EnableResult } from './subscribe';

export interface PushController {
  view: PushView;
  permission: PushPermission;
  support: PushSupport;
  busy: boolean;
  /** The last turn-on or turn-off failed. */
  failed: boolean;
  enable: () => void;
  disable: () => void;
}

function readPermission(): PushPermission {
  return 'Notification' in window ? Notification.permission : 'default';
}

/** The notification row's state, read from the browser and the PC, with its two actions. */
export function usePushNotifications(): PushController {
  const [support] = useState(readSupport);
  const [permission, setPermission] = useState<PushPermission>(readPermission);
  const [status, setStatus] = useState<PushStatus | null>(null);
  const [publicKey, setPublicKey] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const next = await getPushStatus();
      // A subscription kept on the PC is only useful while the browser still holds its own.
      const held = next.subscribed === true ? await hasBrowserSubscription() : false;
      setStatus({ ...next, subscribed: held });
      setPublicKey(next.publicKey ?? null);
    } catch {
      setStatus(null);
    }
    setPermission(readPermission());
  }, []);
  useEffect(() => {
    void refresh();
  }, [refresh]);

  // A turn-on or turn-off finishes only once the status is read again, so `busy` covers both.
  const enabling = useWrite<EnableResult, string>({
    mutationKey: ['push-enable'],
    mutationFn: (key) => enablePush(key),
    onSuccess: (result) => setFailed(result === 'failed'),
    onSettled: refresh,
  });
  const disabling = useWrite<boolean>({
    mutationKey: ['push-disable'],
    mutationFn: () => disablePush(),
    onSuccess: (done) => setFailed(!done),
    onSettled: refresh,
  });
  const busy = enabling.isPending || disabling.isPending;

  const enable = () => {
    if (publicKey === null || enabling.isWriting() || disabling.isWriting()) return;
    setFailed(false);
    enabling.mutate(publicKey);
  };

  const disable = () => {
    if (enabling.isWriting() || disabling.isWriting()) return;
    setFailed(false);
    disabling.mutate();
  };

  return {
    view: pushView(support, permission, status),
    permission,
    support,
    busy,
    failed,
    enable,
    disable,
  };
}
